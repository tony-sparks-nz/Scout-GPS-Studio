import { useState, useEffect, useRef } from 'react';
import type { GpsSourceStatus, OptimizeStatus, PerformanceSnapshot } from '../hooks/useTauri';
import { startOptimize, getOptimizeStatus, abortOptimize } from '../hooks/useTauri';

interface OptimizePanelProps {
  status: GpsSourceStatus | null;
  isUblox: boolean;
}

const PHASE_LABELS: Record<string, string> = {
  idle: 'Ready',
  identifying_chip: 'Identifying chip...',
  collecting_baseline: 'Collecting baseline...',
  applying_profile: 'Applying marine profile...',
  stabilizing: 'Stabilizing...',
  collecting_result: 'Collecting results...',
  complete: 'Complete',
  error: 'Error',
};

function formatPct(val: number): string {
  const sign = val >= 0 ? '+' : '';
  return `${sign}${val.toFixed(1)}%`;
}

function MetricRow({
  label,
  before,
  after,
  improvementPct,
  lowerIsBetter,
}: {
  label: string;
  before: string;
  after: string;
  improvementPct: number;
  lowerIsBetter?: boolean;
}) {
  const isImproved = lowerIsBetter ? improvementPct > 0 : improvementPct > 0;
  const color = Math.abs(improvementPct) < 0.5 ? '#8888aa' : isImproved ? '#00ff41' : '#ff3333';

  return (
    <div className="opt-metric-row">
      <span className="opt-metric-label">{label}</span>
      <span className="opt-metric-before">{before}</span>
      <span className="opt-metric-after">{after}</span>
      <span className="opt-metric-change" style={{ color }}>
        {formatPct(improvementPct)}
      </span>
    </div>
  );
}

function SnapshotTable({ before, after, report }: {
  before: PerformanceSnapshot;
  after: PerformanceSnapshot;
  report: OptimizeStatus['report'];
}) {
  if (!report) return null;

  return (
    <div className="opt-results">
      <div className="opt-metric-header">
        <span className="opt-metric-label">Metric</span>
        <span className="opt-metric-before">Before</span>
        <span className="opt-metric-after">After</span>
        <span className="opt-metric-change">Change</span>
      </div>
      <MetricRow
        label="HDOP"
        before={before.avg_hdop.toFixed(2)}
        after={after.avg_hdop.toFixed(2)}
        improvementPct={report.hdop_improvement_pct}
        lowerIsBetter
      />
      <MetricRow
        label="Satellites"
        before={before.avg_satellites.toFixed(1)}
        after={after.avg_satellites.toFixed(1)}
        improvementPct={report.satellite_improvement_pct}
      />
      <MetricRow
        label="Avg SNR"
        before={`${before.avg_snr.toFixed(1)} dB`}
        after={`${after.avg_snr.toFixed(1)} dB`}
        improvementPct={report.snr_improvement_pct}
      />
      <div className="opt-metric-row">
        <span className="opt-metric-label">Constellations</span>
        <span className="opt-metric-before">{before.constellation_count}</span>
        <span className="opt-metric-after">{after.constellation_count}</span>
        <span
          className="opt-metric-change"
          style={{
            color:
              report.constellation_improvement > 0
                ? '#00ff41'
                : report.constellation_improvement < 0
                  ? '#ff3333'
                  : '#8888aa',
          }}
        >
          {report.constellation_improvement > 0 ? '+' : ''}
          {report.constellation_improvement}
        </span>
      </div>
    </div>
  );
}

export function OptimizePanel({ status, isUblox }: OptimizePanelProps) {
  const [optStatus, setOptStatus] = useState<OptimizeStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isConnected =
    status?.status === 'connected' || status?.status === 'receiving_data';
  const isActive =
    optStatus?.phase !== undefined &&
    optStatus.phase !== 'idle' &&
    optStatus.phase !== 'complete' &&
    optStatus.phase !== 'error';

  // Poll optimization status when active
  useEffect(() => {
    if (!isActive) {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
      return;
    }

    const poll = async () => {
      try {
        const s = await getOptimizeStatus();
        setOptStatus(s);
        if (s.error) setError(s.error);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    };

    poll();
    pollRef.current = setInterval(poll, 500);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [isActive]);

  const handleStart = async () => {
    setError(null);
    try {
      await startOptimize();
      // Immediately poll to get initial status
      const s = await getOptimizeStatus();
      setOptStatus(s);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleAbort = async () => {
    try {
      await abortOptimize();
      setOptStatus(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const progressPct =
    optStatus && optStatus.phase_duration_seconds > 0
      ? Math.min(
          (optStatus.progress_seconds / optStatus.phase_duration_seconds) * 100,
          100
        )
      : 0;

  return (
    <div className="hw-card optimize-panel">
      <h3>Marine Optimization</h3>

      {/* Chip info (when identified) */}
      {optStatus?.chip_info && (
        <div className="opt-chip-info">
          <span className="opt-chip-name">{optStatus.chip_info.chip_name}</span>
          <span className="opt-chip-detail">
            {optStatus.chip_info.series === 'series7'
              ? 'Series 7'
              : optStatus.chip_info.series === 'series8'
                ? 'Series 8'
                : 'Unknown'}{' '}
            — FW: {optStatus.chip_info.sw_version.substring(0, 20)}
          </span>
        </div>
      )}

      {/* Progress during optimization */}
      {isActive && optStatus && (
        <div className="opt-progress">
          <div className="opt-phase-label">
            {PHASE_LABELS[optStatus.phase] || optStatus.phase}
          </div>
          <div className="scan-progress-bar-container">
            <div
              className="scan-progress-bar"
              style={{ width: `${progressPct}%` }}
            />
            <div className="scan-progress-label">
              {Math.round(optStatus.progress_seconds)}s /{' '}
              {Math.round(optStatus.phase_duration_seconds)}s
            </div>
          </div>
        </div>
      )}

      {/* Profile applied label */}
      {optStatus?.report && (
        <div className="opt-profile-label">
          Profile: {optStatus.report.profile_applied}
        </div>
      )}

      {/* Before/After comparison table */}
      {optStatus?.phase === 'complete' &&
        optStatus.report &&
        optStatus.baseline_snapshot && (
          <SnapshotTable
            before={optStatus.report.before}
            after={optStatus.report.after}
            report={optStatus.report}
          />
        )}

      {/* Error display */}
      {(error || optStatus?.error) && (
        <div className="error-msg" style={{ marginTop: 8 }}>
          {error || optStatus?.error}
        </div>
      )}

      {/* Action buttons */}
      <div className="opt-actions">
        {!isActive ? (
          <button
            className="btn btn-primary"
            disabled={!isConnected || !isUblox}
            onClick={handleStart}
          >
            Optimize for Marine
          </button>
        ) : (
          <button className="btn btn-secondary" onClick={handleAbort}>
            Abort
          </button>
        )}
        {!isConnected && (
          <span className="opt-hint">Connect a GPS device first</span>
        )}
        {isConnected && !isUblox && (
          <span className="opt-hint">Requires u-blox GPS</span>
        )}
      </div>
    </div>
  );
}

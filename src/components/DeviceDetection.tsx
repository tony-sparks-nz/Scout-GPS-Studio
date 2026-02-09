import { useState, useEffect, useCallback, useRef } from 'react';
import {
  listSerialPorts,
  testGpsPort,
  connectGps,
  disconnectGps,
  type DetectedPort,
  type GpsSourceStatus,
} from '../hooks/useTauri';

const BAUD_RATES = [4800, 9600, 115200];

interface FoundGps {
  port: DetectedPort;
  baud: number;
}

interface DeviceDetectionProps {
  status: GpsSourceStatus | null;
  onConnected: (port: DetectedPort, baud: number) => void;
}

export function DeviceDetection({ status, onConnected }: DeviceDetectionProps) {
  const [ports, setPorts] = useState<DetectedPort[]>([]);
  const [detecting, setDetecting] = useState(false);
  const [scanProgress, setScanProgress] = useState<string>('');
  const [scanStep, setScanStep] = useState<{ current: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [connectedPort, setConnectedPort] = useState<DetectedPort | null>(null);
  const [connectedBaud, setConnectedBaud] = useState<number>(4800);
  const [otherGpsDevices, setOtherGpsDevices] = useState<FoundGps[]>([]);
  const [manualPort, setManualPort] = useState('');
  const [manualBaud, setManualBaud] = useState(4800);
  const [showManual, setShowManual] = useState(false);
  const abortRef = useRef(false);
  const connectedRef = useRef(false);

  const refreshPorts = useCallback(async () => {
    try {
      const p = await listSerialPorts();
      setPorts(p);
      if (p.length > 0 && !manualPort) {
        setManualPort(p[0].port_name);
      }
    } catch (e) {
      console.debug('Failed to list ports:', e);
    }
  }, [manualPort]);

  const handleAutoDetect = useCallback(async () => {
    setDetecting(true);
    setError(null);
    setScanProgress('Enumerating serial ports...');
    setScanStep(null);
    setOtherGpsDevices([]);
    abortRef.current = false;
    connectedRef.current = false;

    try {
      const allPorts = await listSerialPorts();
      setPorts(allPorts);

      if (allPorts.length === 0) {
        setError('No serial ports found');
        setShowManual(true);
        return;
      }

      // Sort: likely GPS devices first
      const sorted = [...allPorts].sort((a, b) =>
        (a.is_likely_gps === b.is_likely_gps) ? 0 : a.is_likely_gps ? -1 : 1
      );

      setScanProgress(`Found ${sorted.length} port${sorted.length !== 1 ? 's' : ''}. Testing...`);

      const totalSteps = sorted.length * BAUD_RATES.length;
      let step = 0;
      const foundDevices: FoundGps[] = [];

      for (const port of sorted) {
        let portHasGps = false;
        for (const baud of BAUD_RATES) {
          if (abortRef.current) {
            setError('Scan cancelled');
            setShowManual(true);
            return;
          }

          step++;
          setScanStep({ current: step, total: totalSteps });
          setScanProgress(
            `Testing ${port.port_name} @ ${baud} baud` +
            (port.is_likely_gps ? ' [likely GPS]' : '') +
            (port.manufacturer ? ` (${port.manufacturer})` : '')
          );

          try {
            const isGps = await testGpsPort(port.port_name, baud);
            if (isGps) {
              if (!connectedRef.current) {
                // First GPS found — connect immediately
                connectedRef.current = true;
                setScanProgress(`GPS found on ${port.port_name} @ ${baud}. Connecting...`);
                setConnectedPort(port);
                setConnectedBaud(baud);
                await connectGps(port.port_name, baud);
                onConnected(port, baud);
                setScanProgress(`Connected to ${port.port_name}. Scanning for more GPS devices...`);
              } else {
                // Additional GPS found — add to switchable list
                foundDevices.push({ port, baud });
                setOtherGpsDevices([...foundDevices]);
              }
              portHasGps = true;
              break; // Don't test other baud rates for this port
            }
          } catch {
            // Port test failed — skip
          }
        }
        // If this port already found GPS, skip remaining baud rates (handled by break above)
        // but also skip remaining steps in the counter for this port
        if (portHasGps) {
          step = Math.min(step + (BAUD_RATES.length - 1), totalSteps);
        }
      }

      if (!connectedRef.current) {
        setError(`No GPS device detected on ${sorted.length} port${sorted.length !== 1 ? 's' : ''}`);
        setShowManual(true);
        if (allPorts.length > 0 && !manualPort) {
          setManualPort(allPorts[0].port_name);
        }
      }
    } catch (e: any) {
      setError(e.message || 'Scan failed');
      setShowManual(true);
      await refreshPorts();
    } finally {
      setDetecting(false);
      setScanStep(null);
    }
  }, [onConnected, refreshPorts, manualPort]);

  const handleSwitchGps = useCallback(async (found: FoundGps) => {
    try {
      await disconnectGps();
      await connectGps(found.port.port_name, found.baud);
      if (connectedPort) {
        setOtherGpsDevices(prev => {
          const updated = prev.filter(d => d.port.port_name !== found.port.port_name);
          updated.push({ port: connectedPort, baud: connectedBaud });
          return updated;
        });
      }
      setConnectedPort(found.port);
      setConnectedBaud(found.baud);
      onConnected(found.port, found.baud);
    } catch (e: any) {
      setError(e.message || 'Failed to switch GPS');
    }
  }, [connectedPort, connectedBaud, onConnected]);

  const handleRescan = useCallback(async () => {
    try {
      await disconnectGps();
    } catch {
      // ignore
    }
    setConnectedPort(null);
    setOtherGpsDevices([]);
    setShowManual(false);
    setError(null);
    connectedRef.current = false;
    handleAutoDetect();
  }, [handleAutoDetect]);

  const handleManualConnect = useCallback(async () => {
    setDetecting(true);
    setError(null);
    try {
      await connectGps(manualPort, manualBaud);
      const foundPort = ports.find(p => p.port_name === manualPort) || {
        port_name: manualPort,
        port_type: 'Unknown',
        manufacturer: null,
        product: null,
        serial_number: null,
        vid: null,
        pid: null,
        is_likely_gps: false,
      };
      setConnectedPort(foundPort);
      setConnectedBaud(manualBaud);
      onConnected(foundPort, manualBaud);
    } catch (e: any) {
      setError(e.message || 'Failed to connect');
    } finally {
      setDetecting(false);
    }
  }, [manualPort, manualBaud, ports, onConnected]);

  const handleDisconnect = useCallback(async () => {
    try {
      await disconnectGps();
      setConnectedPort(null);
      setOtherGpsDevices([]);
    } catch (e) {
      console.error('Disconnect failed:', e);
    }
  }, []);

  // Auto-detect on mount
  useEffect(() => {
    handleAutoDetect();
  }, []);

  const isConnected = status?.status === 'connected' || status?.status === 'receiving_data';
  const isReceiving = status?.status === 'receiving_data';

  const statusColor = isReceiving
    ? '#00ff41'
    : isConnected
      ? '#00aaff'
      : status?.status === 'connecting'
        ? '#ffaa00'
        : status?.status === 'error'
          ? '#ff3333'
          : '#666';

  return connectedPort ? (
    <div className="connection-bar">
      <div className="conn-bar-left">
        <div className="status-dot" style={{ backgroundColor: statusColor }} />
        <span className="conn-bar-status">
          {isReceiving ? 'Receiving' : isConnected ? 'Connected' : status?.status || 'Unknown'}
        </span>
        <span className="conn-bar-port">{connectedPort.port_name}</span>
        <span className="conn-bar-detail">{connectedBaud} baud</span>
        {connectedPort.manufacturer && (
          <span className="conn-bar-detail">{connectedPort.manufacturer}</span>
        )}
        {connectedPort.serial_number && (
          <span className="conn-bar-serial">{connectedPort.serial_number}</span>
        )}
        {status?.sentences_received ? (
          <span className="conn-bar-detail">{status.sentences_received} sentences</span>
        ) : null}
      </div>

      <div className="conn-bar-right">
        {/* Scanning progress while connected */}
        {detecting && scanStep && (
          <div className="conn-bar-scan">
            <span className="scan-status-small">{scanProgress}</span>
            <div className="scan-progress-bar-container small">
              <div
                className="scan-progress-bar"
                style={{ width: `${(scanStep.current / scanStep.total) * 100}%` }}
              />
            </div>
          </div>
        )}

        {/* Other GPS devices */}
        {otherGpsDevices.length > 0 && otherGpsDevices.map((found) => (
          <button
            key={found.port.port_name}
            className="btn btn-small btn-secondary"
            onClick={() => handleSwitchGps(found)}
            title={`Switch to ${found.port.port_name} @ ${found.baud}`}
          >
            {found.port.port_name}
          </button>
        ))}

        <button className="btn btn-small btn-secondary" onClick={handleDisconnect}>
          Disconnect
        </button>
        <button className="btn btn-small btn-primary" onClick={handleRescan} disabled={detecting}>
          Rescan
        </button>
      </div>
    </div>
  ) : (
    <div className="connection-bar connection-bar-scanning">
      {detecting ? (
        <div className="conn-bar-detecting">
          <div className="spinner spinner-small" />
          <span className="scan-status">{scanProgress}</span>
          {scanStep && (
            <div className="scan-progress-bar-container">
              <div
                className="scan-progress-bar"
                style={{ width: `${(scanStep.current / scanStep.total) * 100}%` }}
              />
              <span className="scan-progress-label">
                {scanStep.current} / {scanStep.total}
              </span>
            </div>
          )}
          <button className="btn btn-small btn-secondary" onClick={() => { abortRef.current = true; }}>
            Cancel
          </button>
        </div>
      ) : (
        <div className="conn-bar-disconnected">
          <button className="btn btn-primary" onClick={handleAutoDetect}>
            Auto-Detect GPS
          </button>

          {error && <span className="error-msg-inline">{error}</span>}

          {showManual && (
            <div className="manual-row">
              <select
                value={manualPort}
                onChange={(e) => setManualPort(e.target.value)}
              >
                {ports.map((p) => (
                  <option key={p.port_name} value={p.port_name}>
                    {p.port_name} {p.manufacturer ? `(${p.manufacturer})` : ''} {p.is_likely_gps ? ' [GPS]' : ''}
                  </option>
                ))}
                {ports.length === 0 && <option value="">No ports found</option>}
              </select>
              <select
                value={manualBaud}
                onChange={(e) => setManualBaud(Number(e.target.value))}
              >
                {[4800, 9600, 19200, 38400, 57600, 115200].map((b) => (
                  <option key={b} value={b}>{b}</option>
                ))}
              </select>
              <button
                className="btn btn-primary btn-small"
                onClick={handleManualConnect}
                disabled={!manualPort}
              >
                Connect
              </button>
              <button className="btn btn-secondary btn-small" onClick={refreshPorts}>
                Refresh
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

import type { GpsData, SatelliteInfo, TestCriteria } from '../hooks/useTauri';

interface LiveDashboardProps {
  gpsData: GpsData | null;
  criteria: TestCriteria | null;
}

export function LiveDashboard({ gpsData, criteria }: LiveDashboardProps) {
  const getSnrColor = (snr: number | null): string => {
    if (snr === null) return '#666';
    if (snr >= 40) return '#00ff41';
    if (snr >= 30) return '#00ccff';
    if (snr >= 20) return '#ffaa00';
    return '#ff3333';
  };

  const getDopColor = (value: number | null, max: number | undefined): string => {
    if (value === null) return '#666';
    if (max && value <= max) return '#00ff41';
    if (value <= 2.0) return '#00ff41';
    if (value <= 5.0) return '#ffaa00';
    return '#ff3333';
  };

  const getFixQualityText = (quality: number | null): string => {
    if (quality === null) return 'Unknown';
    switch (quality) {
      case 0: return 'No Fix';
      case 1: return 'GPS Fix';
      case 2: return 'DGPS Fix';
      case 3: return 'PPS Fix';
      case 4: return 'RTK Fixed';
      case 5: return 'RTK Float';
      case 6: return 'Estimated';
      case 7: return 'Manual';
      case 8: return 'Simulation';
      default: return `Unknown (${quality})`;
    }
  };

  // Group satellites by constellation
  const groupedSatellites = (gpsData?.satellites_info || []).reduce<Record<string, SatelliteInfo[]>>(
    (acc, sat) => {
      const key = sat.constellation || 'Unknown';
      if (!acc[key]) acc[key] = [];
      acc[key].push(sat);
      return acc;
    },
    {}
  );

  const satCount = gpsData?.satellites ?? 0;
  const satColor = criteria
    ? satCount >= criteria.min_satellites ? '#00ff41' : '#ff3333'
    : '#ccc';

  return (
    <section className="panel live-dashboard">
      <h2>Live GPS Data</h2>

      {/* Summary metrics */}
      <div className="metrics-grid">
        <div className="metric">
          <span className="metric-label">Fix</span>
          <span className="metric-value">
            {gpsData?.fix_type || getFixQualityText(gpsData?.fix_quality ?? null)}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">Satellites</span>
          <span className="metric-value" style={{ color: satColor }}>
            {satCount}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">HDOP</span>
          <span
            className="metric-value"
            style={{ color: getDopColor(gpsData?.hdop ?? null, criteria?.max_hdop) }}
          >
            {gpsData?.hdop?.toFixed(1) ?? '-'}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">VDOP</span>
          <span className="metric-value" style={{ color: getDopColor(gpsData?.vdop ?? null, undefined) }}>
            {gpsData?.vdop?.toFixed(1) ?? '-'}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">PDOP</span>
          <span
            className="metric-value"
            style={{ color: getDopColor(gpsData?.pdop ?? null, criteria?.max_pdop) }}
          >
            {gpsData?.pdop?.toFixed(1) ?? '-'}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">Altitude</span>
          <span className="metric-value">
            {gpsData?.altitude != null ? `${gpsData.altitude.toFixed(1)}m` : '-'}
          </span>
        </div>
      </div>

      {/* Constellation badges */}
      {Object.keys(groupedSatellites).length > 0 && (
        <div className="constellation-badges">
          {Object.entries(groupedSatellites)
            .sort((a, b) => b[1].length - a[1].length)
            .map(([constellation, sats]) => (
              <div key={constellation} className="constellation-badge">
                <span className="badge-name">{constellation}</span>
                <span className="badge-count">{sats.length}</span>
              </div>
            ))}
        </div>
      )}

      {/* Satellite signal bars */}
      <div className="satellite-signals">
        {Object.keys(groupedSatellites).length === 0 ? (
          <p className="no-data">Waiting for satellite data...</p>
        ) : (
          Object.entries(groupedSatellites).map(([constellation, sats]) => (
            <div key={constellation} className="constellation-group">
              <h4>{constellation} ({sats.length})</h4>
              <div className="satellite-bars">
                {sats
                  .sort((a, b) => (b.snr ?? 0) - (a.snr ?? 0))
                  .map((sat) => (
                    <div key={`${constellation}-${sat.prn}`} className="satellite-row">
                      <span className="sat-prn">PRN {sat.prn}</span>
                      <span className="sat-angles">
                        El:{sat.elevation?.toFixed(0) ?? '-'}° Az:{sat.azimuth?.toFixed(0) ?? '-'}°
                      </span>
                      <div className="signal-bar-track">
                        <div
                          className="signal-bar-fill"
                          style={{
                            width: `${Math.min((sat.snr ?? 0) / 50 * 100, 100)}%`,
                            backgroundColor: getSnrColor(sat.snr),
                          }}
                        />
                      </div>
                      <span className="sat-snr" style={{ color: getSnrColor(sat.snr) }}>
                        {sat.snr?.toFixed(0) ?? '-'} dB
                      </span>
                    </div>
                  ))}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Constellation summary */}
      <div className="constellation-summary">
        <h4>Constellations</h4>
        {Object.keys(groupedSatellites).length === 0 ? (
          <p className="no-data">No constellations</p>
        ) : (
          <>
            <div className="constellation-summary-header">
              <span>System</span>
              <span>Visible</span>
              <span>Tracking</span>
            </div>
            {Object.entries(groupedSatellites)
              .sort((a, b) => b[1].length - a[1].length)
              .map(([constellation, sats]) => {
                const tracking = sats.filter(s => s.snr !== null && s.snr > 0).length;
                return (
                  <div key={constellation} className="constellation-summary-row">
                    <span className="cs-name">{constellation}</span>
                    <span className="cs-count">{sats.length}</span>
                    <span className="cs-tracking">{tracking}</span>
                  </div>
                );
              })}
            <div className="constellation-summary-total">
              <span className="cs-name">Total</span>
              <span className="cs-count">
                {Object.values(groupedSatellites).reduce((sum, sats) => sum + sats.length, 0)}
              </span>
              <span className="cs-tracking">
                {Object.values(groupedSatellites).reduce(
                  (sum, sats) => sum + sats.filter(s => s.snr !== null && s.snr > 0).length, 0
                )}
              </span>
            </div>
          </>
        )}
      </div>
    </section>
  );
}

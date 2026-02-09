import { useMemo } from 'react';
import type { GpsData, DetectedPort, GpsSourceStatus } from '../hooks/useTauri';

interface HardwareInfoProps {
  gpsData: GpsData | null;
  connectedPort: DetectedPort | null;
  connectedBaud: number | null;
  status: GpsSourceStatus | null;
}

// Known USB VID database for GPS-related devices
function lookupVendor(vid: number | null): string {
  if (vid === null) return 'Unknown';
  const vendors: Record<number, string> = {
    0x1546: 'u-blox AG',
    0x067b: 'Prolific Technology',
    0x0403: 'FTDI (Future Technology Devices)',
    0x10c4: 'Silicon Labs (CP210x)',
    0x1a86: 'QinHeng Electronics (CH340)',
    0x2341: 'Arduino',
    0x239a: 'Adafruit',
    0x303a: 'Espressif',
    0x04b4: 'Cypress Semiconductor',
  };
  return vendors[vid] || `Unknown (0x${vid.toString(16).padStart(4, '0')})`;
}

function lookupProduct(vid: number | null, pid: number | null): string {
  if (vid === null || pid === null) return 'Unknown';
  const products: Record<string, string> = {
    '1546:01a7': 'u-blox 7 (GPS/GLONASS)',
    '1546:01a8': 'u-blox 8 / NEO-M8N',
    '1546:01a9': 'u-blox NEO-M9N',
    '067b:2303': 'PL2303 USB-to-Serial',
    '067b:23a3': 'PL2303GC USB-to-Serial',
    '067b:23c3': 'PL2303GS USB-to-Serial',
    '0403:6001': 'FT232R USB UART',
    '0403:6010': 'FT2232 Dual UART',
    '0403:6014': 'FT232H Single HS USB-UART',
    '0403:6015': 'FT-X Series USB UART',
    '10c4:ea60': 'CP2102/CP2109 USB-to-UART',
    '10c4:ea70': 'CP2105 Dual USB-to-UART',
    '10c4:ea71': 'CP2108 Quad USB-to-UART',
    '1a86:7523': 'CH340 USB-to-Serial',
    '1a86:5523': 'CH341 USB-to-Serial',
    '1a86:55d4': 'CH9102 USB-to-Serial',
  };
  const key = `${vid.toString(16).padStart(4, '0')}:${pid.toString(16).padStart(4, '0')}`;
  return products[key] || `PID 0x${pid.toString(16).padStart(4, '0')}`;
}

function formatCoord(val: number | null, pos: string, neg: string): string {
  if (val === null) return '-';
  const dir = val >= 0 ? pos : neg;
  const abs = Math.abs(val);
  const deg = Math.floor(abs);
  const min = (abs - deg) * 60;
  return `${deg}° ${min.toFixed(4)}' ${dir}  (${val.toFixed(6)}°)`;
}

export function HardwareInfo({ gpsData, connectedPort, connectedBaud, status }: HardwareInfoProps) {
  const stats = useMemo(() => {
    if (!gpsData) return null;

    const sats = gpsData.satellites_info || [];
    const withSnr = sats.filter(s => s.snr !== null && s.snr > 0);
    const allSnr = withSnr.map(s => s.snr!);

    // Per-constellation stats
    const constellations: Record<string, { count: number; tracked: number; avgSnr: number; maxSnr: number; minSnr: number; snrs: number[] }> = {};
    for (const sat of sats) {
      const c = sat.constellation || 'Unknown';
      if (!constellations[c]) {
        constellations[c] = { count: 0, tracked: 0, avgSnr: 0, maxSnr: 0, minSnr: 99, snrs: [] };
      }
      constellations[c].count++;
      if (sat.snr !== null && sat.snr > 0) {
        constellations[c].tracked++;
        constellations[c].snrs.push(sat.snr);
        constellations[c].maxSnr = Math.max(constellations[c].maxSnr, sat.snr);
        constellations[c].minSnr = Math.min(constellations[c].minSnr, sat.snr);
      }
    }
    for (const c of Object.values(constellations)) {
      c.avgSnr = c.snrs.length > 0 ? c.snrs.reduce((a, b) => a + b, 0) / c.snrs.length : 0;
    }

    // Signal quality distribution
    const excellent = withSnr.filter(s => s.snr! >= 40).length;
    const good = withSnr.filter(s => s.snr! >= 30 && s.snr! < 40).length;
    const fair = withSnr.filter(s => s.snr! >= 20 && s.snr! < 30).length;
    const weak = withSnr.filter(s => s.snr! < 20).length;

    // Elevation distribution
    const highElev = sats.filter(s => s.elevation !== null && s.elevation >= 45).length;
    const midElev = sats.filter(s => s.elevation !== null && s.elevation >= 15 && s.elevation < 45).length;
    const lowElev = sats.filter(s => s.elevation !== null && s.elevation > 0 && s.elevation < 15).length;

    return {
      totalVisible: sats.length,
      totalTracked: withSnr.length,
      constellationCount: Object.keys(constellations).length,
      constellations,
      globalAvgSnr: allSnr.length > 0 ? allSnr.reduce((a, b) => a + b, 0) / allSnr.length : 0,
      globalMaxSnr: allSnr.length > 0 ? Math.max(...allSnr) : 0,
      globalMinSnr: allSnr.length > 0 ? Math.min(...allSnr) : 0,
      signalDist: { excellent, good, fair, weak },
      elevDist: { high: highElev, mid: midElev, low: lowElev },
      hAcc: gpsData.hdop !== null ? (gpsData.hdop * 2.5).toFixed(1) : null,
      vAcc: gpsData.vdop !== null ? (gpsData.vdop * 3.5).toFixed(1) : null,
      pAcc: gpsData.pdop !== null ? (gpsData.pdop * 3.0).toFixed(1) : null,
    };
  }, [gpsData]);

  const isConnected = status?.status === 'connected' || status?.status === 'receiving_data';

  return (
    <div className="hw-column">
      {/* Device */}
      <div className="hw-card">
        <h3>Device</h3>
        <div className="hw-grid">
          <span className="hw-label">Port</span>
          <span className="hw-value mono">{connectedPort?.port_name || '-'} @ {connectedBaud || '-'}</span>

          <span className="hw-label">Type</span>
          <span className="hw-value">{connectedPort?.port_type || '-'}</span>

          <span className="hw-label">VID / PID</span>
          <span className="hw-value mono">
            {connectedPort?.vid != null ? `0x${connectedPort.vid.toString(16).padStart(4, '0').toUpperCase()}` : '-'}
            {' / '}
            {connectedPort?.pid != null ? `0x${connectedPort.pid.toString(16).padStart(4, '0').toUpperCase()}` : '-'}
          </span>

          <span className="hw-label">Vendor</span>
          <span className="hw-value">{lookupVendor(connectedPort?.vid ?? null)}</span>

          <span className="hw-label">Product</span>
          <span className="hw-value">{lookupProduct(connectedPort?.vid ?? null, connectedPort?.pid ?? null)}</span>

          <span className="hw-label">Manufacturer</span>
          <span className="hw-value">{connectedPort?.manufacturer || '-'}</span>

          <span className="hw-label">Serial</span>
          <span className="hw-value mono accent">{connectedPort?.serial_number || '-'}</span>
        </div>
      </div>

      {/* Connection */}
      <div className="hw-card">
        <h3>Connection</h3>
        <div className="hw-grid">
          <span className="hw-label">State</span>
          <span className={`hw-value ${isConnected ? 'pass' : 'fail'}`}>
            {status?.status?.replace(/_/g, ' ').toUpperCase() || 'DISCONNECTED'}
          </span>

          <span className="hw-label">Sentences</span>
          <span className="hw-value mono">{status?.sentences_received ?? 0}</span>

          <span className="hw-label">Data Rate</span>
          <span className="hw-value mono">
            {connectedBaud ? `${connectedBaud} baud (${(connectedBaud / 10).toFixed(0)} B/s)` : '-'}
          </span>

          <span className="hw-label">Last Fix</span>
          <span className="hw-value mono">{status?.last_fix_time || '-'}</span>

          <span className="hw-label">Last Error</span>
          <span className="hw-value" style={{ color: status?.last_error ? '#ff3333' : undefined }}>
            {status?.last_error || 'None'}
          </span>
        </div>
      </div>

      {/* Position */}
      <div className="hw-card">
        <h3>Position</h3>
        <div className="hw-grid">
          <span className="hw-label">Latitude</span>
          <span className="hw-value mono">{formatCoord(gpsData?.latitude ?? null, 'N', 'S')}</span>

          <span className="hw-label">Longitude</span>
          <span className="hw-value mono">{formatCoord(gpsData?.longitude ?? null, 'E', 'W')}</span>

          <span className="hw-label">Altitude</span>
          <span className="hw-value mono">{gpsData?.altitude != null ? `${gpsData.altitude.toFixed(1)} m MSL` : '-'}</span>

          <span className="hw-label">Speed</span>
          <span className="hw-value mono">
            {gpsData?.speed_knots != null
              ? `${gpsData.speed_knots.toFixed(1)} kn (${(gpsData.speed_knots * 1.852).toFixed(1)} km/h)`
              : '-'}
          </span>

          <span className="hw-label">Course (COG)</span>
          <span className="hw-value mono">{gpsData?.course != null ? `${gpsData.course.toFixed(1)}° T` : '-'}</span>

          <span className="hw-label">Heading (HDG)</span>
          <span className="hw-value mono">{gpsData?.heading != null ? `${gpsData.heading.toFixed(1)}° T` : '-'}</span>

          <span className="hw-label">GPS Time</span>
          <span className="hw-value mono">{gpsData?.timestamp || '-'}</span>
        </div>
      </div>

      {/* Fix & Accuracy */}
      <div className="hw-card">
        <h3>Fix & Accuracy</h3>
        <div className="hw-grid">
          <span className="hw-label">Fix</span>
          <span className="hw-value">{gpsData?.fix_type || 'No Fix'}</span>

          <span className="hw-label">Quality</span>
          <span className="hw-value">
            {gpsData?.fix_quality != null
              ? `${gpsData.fix_quality} (${['No Fix', 'GPS SPS', 'DGPS', 'PPS', 'RTK Fixed', 'RTK Float', 'Estimated', 'Manual', 'Simulation'][gpsData.fix_quality] || 'Unknown'})`
              : '-'}
          </span>

          <span className="hw-label">Sats in Fix</span>
          <span className="hw-value mono">{gpsData?.satellites ?? '-'}</span>

          <span className="hw-label">HDOP</span>
          <span className="hw-value mono">
            {gpsData?.hdop != null ? `${gpsData.hdop.toFixed(2)}` : '-'}
            {stats?.hAcc ? ` (~${stats.hAcc}m)` : ''}
          </span>

          <span className="hw-label">VDOP</span>
          <span className="hw-value mono">
            {gpsData?.vdop != null ? `${gpsData.vdop.toFixed(2)}` : '-'}
            {stats?.vAcc ? ` (~${stats.vAcc}m)` : ''}
          </span>

          <span className="hw-label">PDOP</span>
          <span className="hw-value mono">
            {gpsData?.pdop != null ? `${gpsData.pdop.toFixed(2)}` : '-'}
            {stats?.pAcc ? ` (~${stats.pAcc}m)` : ''}
          </span>
        </div>
      </div>

      {/* Signal Statistics */}
      {stats && (
        <div className="hw-card">
          <h3>Signal Stats</h3>
          <div className="hw-stats-row">
            <div className="hw-stat">
              <span className="hw-stat-val">{stats.totalVisible}</span>
              <span className="hw-stat-lbl">Visible</span>
            </div>
            <div className="hw-stat">
              <span className="hw-stat-val">{stats.totalTracked}</span>
              <span className="hw-stat-lbl">Tracked</span>
            </div>
            <div className="hw-stat">
              <span className="hw-stat-val">{stats.constellationCount}</span>
              <span className="hw-stat-lbl">Systems</span>
            </div>
            <div className="hw-stat">
              <span className="hw-stat-val">{stats.globalAvgSnr.toFixed(0)}</span>
              <span className="hw-stat-lbl">Avg dB</span>
            </div>
            <div className="hw-stat">
              <span className="hw-stat-val">{stats.globalMaxSnr.toFixed(0)}</span>
              <span className="hw-stat-lbl">Peak dB</span>
            </div>
          </div>

          <h4>Signal Quality</h4>
          <div className="signal-dist">
            <div className="dist-bar">
              <span className="dist-label" style={{ color: '#00ff41' }}>&ge;40 dB</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalTracked > 0 ? (stats.signalDist.excellent / stats.totalTracked) * 100 : 0}%`,
                  backgroundColor: '#00ff41'
                }} />
              </div>
              <span className="dist-count">{stats.signalDist.excellent}</span>
            </div>
            <div className="dist-bar">
              <span className="dist-label" style={{ color: '#00ccff' }}>30-39</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalTracked > 0 ? (stats.signalDist.good / stats.totalTracked) * 100 : 0}%`,
                  backgroundColor: '#00ccff'
                }} />
              </div>
              <span className="dist-count">{stats.signalDist.good}</span>
            </div>
            <div className="dist-bar">
              <span className="dist-label" style={{ color: '#ffaa00' }}>20-29</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalTracked > 0 ? (stats.signalDist.fair / stats.totalTracked) * 100 : 0}%`,
                  backgroundColor: '#ffaa00'
                }} />
              </div>
              <span className="dist-count">{stats.signalDist.fair}</span>
            </div>
            <div className="dist-bar">
              <span className="dist-label" style={{ color: '#ff3333' }}>&lt;20</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalTracked > 0 ? (stats.signalDist.weak / stats.totalTracked) * 100 : 0}%`,
                  backgroundColor: '#ff3333'
                }} />
              </div>
              <span className="dist-count">{stats.signalDist.weak}</span>
            </div>
          </div>

          <h4>Elevation</h4>
          <div className="signal-dist">
            <div className="dist-bar">
              <span className="dist-label">High &ge;45°</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalVisible > 0 ? (stats.elevDist.high / stats.totalVisible) * 100 : 0}%`,
                  backgroundColor: '#00ff41'
                }} />
              </div>
              <span className="dist-count">{stats.elevDist.high}</span>
            </div>
            <div className="dist-bar">
              <span className="dist-label">Mid 15-44°</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalVisible > 0 ? (stats.elevDist.mid / stats.totalVisible) * 100 : 0}%`,
                  backgroundColor: '#00ccff'
                }} />
              </div>
              <span className="dist-count">{stats.elevDist.mid}</span>
            </div>
            <div className="dist-bar">
              <span className="dist-label">Low &lt;15°</span>
              <div className="dist-track">
                <div className="dist-fill" style={{
                  width: `${stats.totalVisible > 0 ? (stats.elevDist.low / stats.totalVisible) * 100 : 0}%`,
                  backgroundColor: '#ffaa00'
                }} />
              </div>
              <span className="dist-count">{stats.elevDist.low}</span>
            </div>
          </div>

          <h4>Per-Constellation</h4>
          <table className="constellation-table">
            <thead>
              <tr>
                <th>System</th>
                <th>Vis</th>
                <th>Trk</th>
                <th>Avg</th>
                <th>Max</th>
                <th>Min</th>
              </tr>
            </thead>
            <tbody>
              {Object.entries(stats.constellations)
                .sort((a, b) => b[1].count - a[1].count)
                .map(([name, c]) => (
                  <tr key={name}>
                    <td>{name}</td>
                    <td className="mono">{c.count}</td>
                    <td className="mono">{c.tracked}</td>
                    <td className="mono">{c.avgSnr > 0 ? c.avgSnr.toFixed(0) : '-'}</td>
                    <td className="mono">{c.maxSnr > 0 ? c.maxSnr.toFixed(0) : '-'}</td>
                    <td className="mono">{c.minSnr < 99 ? c.minSnr.toFixed(0) : '-'}</td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

import { useState, useEffect, useCallback } from 'react';
import {
  listSerialPorts,
  autoDetectGps,
  connectGps,
  disconnectGps,
  type DetectedPort,
  type GpsSourceStatus,
} from '../hooks/useTauri';

interface DeviceDetectionProps {
  status: GpsSourceStatus | null;
  onConnected: () => void;
}

export function DeviceDetection({ status, onConnected }: DeviceDetectionProps) {
  const [ports, setPorts] = useState<DetectedPort[]>([]);
  const [detecting, setDetecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connectedPort, setConnectedPort] = useState<DetectedPort | null>(null);
  const [connectedBaud, setConnectedBaud] = useState<number>(4800);
  const [manualPort, setManualPort] = useState('');
  const [manualBaud, setManualBaud] = useState(4800);
  const [showManual, setShowManual] = useState(false);

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
    try {
      const [port, baud] = await autoDetectGps();
      setConnectedPort(port);
      setConnectedBaud(baud);
      await connectGps(port.port_name, baud);
      onConnected();
    } catch (e: any) {
      setError(e.message || 'No GPS device detected');
      setShowManual(true);
      await refreshPorts();
    } finally {
      setDetecting(false);
    }
  }, [onConnected, refreshPorts]);

  const handleManualConnect = useCallback(async () => {
    setDetecting(true);
    setError(null);
    try {
      await connectGps(manualPort, manualBaud);
      const port = ports.find(p => p.port_name === manualPort);
      setConnectedPort(port || {
        port_name: manualPort,
        port_type: 'Unknown',
        manufacturer: null,
        product: null,
        serial_number: null,
        is_likely_gps: false,
      });
      setConnectedBaud(manualBaud);
      onConnected();
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

  return (
    <section className="panel device-detection">
      <h2>1. Device Detection</h2>

      {connectedPort ? (
        <div className="device-info">
          <div className="device-status-row">
            <div className="status-dot" style={{ backgroundColor: statusColor }} />
            <span className="status-text">
              {isReceiving ? 'Receiving Data' : isConnected ? 'Connected' : status?.status || 'Unknown'}
            </span>
            {status?.sentences_received ? (
              <span className="sentence-count">{status.sentences_received} sentences</span>
            ) : null}
          </div>

          <div className="device-details">
            <div className="detail-row">
              <span className="detail-label">Port</span>
              <span className="detail-value">{connectedPort.port_name}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">Baud</span>
              <span className="detail-value">{connectedBaud}</span>
            </div>
            {connectedPort.manufacturer && (
              <div className="detail-row">
                <span className="detail-label">Manufacturer</span>
                <span className="detail-value">{connectedPort.manufacturer}</span>
              </div>
            )}
            {connectedPort.product && (
              <div className="detail-row">
                <span className="detail-label">Product</span>
                <span className="detail-value">{connectedPort.product}</span>
              </div>
            )}
            {connectedPort.serial_number && (
              <div className="detail-row">
                <span className="detail-label">Serial</span>
                <span className="detail-value serial-number">{connectedPort.serial_number}</span>
              </div>
            )}
            <div className="detail-row">
              <span className="detail-label">Type</span>
              <span className="detail-value">{connectedPort.port_type}</span>
            </div>
          </div>

          <button className="btn btn-secondary" onClick={handleDisconnect}>
            Disconnect
          </button>
        </div>
      ) : (
        <div className="device-detect-actions">
          {detecting ? (
            <div className="detecting">
              <div className="spinner" />
              <span>Scanning ports...</span>
            </div>
          ) : (
            <>
              <button className="btn btn-primary" onClick={handleAutoDetect}>
                Auto-Detect GPS
              </button>

              {error && <div className="error-msg">{error}</div>}

              {showManual && (
                <div className="manual-connect">
                  <h4>Manual Connection</h4>
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
                      className="btn btn-primary"
                      onClick={handleManualConnect}
                      disabled={!manualPort}
                    >
                      Connect
                    </button>
                    <button className="btn btn-secondary" onClick={refreshPorts}>
                      Refresh
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </section>
  );
}

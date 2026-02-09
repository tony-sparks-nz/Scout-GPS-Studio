import { useState, useEffect, useCallback } from 'react';
import {
  getGpsData,
  getGpsStatus,
  isTauri,
  type GpsData,
  type GpsSourceStatus,
  type DetectedPort,
} from './hooks/useTauri';
import { DeviceDetection } from './components/DeviceDetection';
import { LiveDashboard } from './components/LiveDashboard';
import { NmeaTraffic } from './components/NmeaTraffic';
import { MapPanel } from './components/MapPanel';
import { HardwareInfo } from './components/HardwareInfo';
import { OptimizePanel } from './components/OptimizePanel';
import './App.css';

function App() {
  const [gpsData, setGpsData] = useState<GpsData | null>(null);
  const [gpsStatus, setGpsStatus] = useState<GpsSourceStatus | null>(null);
  const [connectedPort, setConnectedPort] = useState<DetectedPort | null>(null);
  const [connectedBaud, setConnectedBaud] = useState<number | null>(null);

  // Polling loop for GPS data
  useEffect(() => {
    if (!isTauri()) return;

    const poll = async () => {
      try {
        const [data, status] = await Promise.all([
          getGpsData(),
          getGpsStatus(),
        ]);
        setGpsData(data);
        setGpsStatus(status);
      } catch (error) {
        console.debug('Poll error:', error);
      }
    };

    poll();
    const interval = setInterval(poll, 500);
    return () => clearInterval(interval);
  }, []);

  const handleConnected = useCallback((port: DetectedPort, baud: number) => {
    setConnectedPort(port);
    setConnectedBaud(baud);
  }, []);

  return (
    <div className="app">
      {/* Header */}
      <header className="app-header">
        <div className="header-left">
          <h1>Vortex Marine Limited - GPS Studio</h1>
          <span className="version">v3.42</span>
          <span className="copyright">&copy; 2025 Vortex Marine Limited. All rights reserved. Licensed software — unauthorized copying or distribution is strictly prohibited.</span>
        </div>
      </header>

      {/* Connection bar */}
      <DeviceDetection status={gpsStatus} onConnected={handleConnected} />

      {/* Main content: 3-column layout */}
      <main className="app-main">
        <div className="main-content">
          <div className="col-left">
            <LiveDashboard gpsData={gpsData} criteria={null} />
          </div>
          <MapPanel gpsData={gpsData} />
          <div className="col-right">
            <OptimizePanel
              status={gpsStatus}
              isUblox={connectedPort?.vid === 0x1546}
            />
            <HardwareInfo
              gpsData={gpsData}
              connectedPort={connectedPort}
              connectedBaud={connectedBaud}
              status={gpsStatus}
            />
          </div>
        </div>
        <NmeaTraffic visible={true} />
      </main>
    </div>
  );
}

export default App;

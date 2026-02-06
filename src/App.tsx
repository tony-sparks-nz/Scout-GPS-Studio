import { useState, useEffect, useCallback } from 'react';
import {
  getGpsData,
  getGpsStatus,
  getTestStatus,
  getTestCriteria,
  disconnectGps,
  isTauri,
  type GpsData,
  type GpsSourceStatus,
  type TestResult,
  type TestCriteria,
} from './hooks/useTauri';
import { DeviceDetection } from './components/DeviceDetection';
import { LiveDashboard } from './components/LiveDashboard';
import { TestRunner } from './components/TestRunner';
import { NmeaTraffic } from './components/NmeaTraffic';
import { TestHistory } from './components/TestHistory';
import { ConfigPanel } from './components/ConfigPanel';
import './App.css';

function App() {
  const [gpsData, setGpsData] = useState<GpsData | null>(null);
  const [gpsStatus, setGpsStatus] = useState<GpsSourceStatus | null>(null);
  const [testResult, setTestResult] = useState<TestResult | null>(null);
  const [criteria, setCriteria] = useState<TestCriteria | null>(null);
  const [testHistory, setTestHistory] = useState<TestResult[]>([]);
  const [showNmea, setShowNmea] = useState(false);
  const [showConfig, setShowConfig] = useState(false);
  const [connected, setConnected] = useState(false);

  // Load criteria on mount
  useEffect(() => {
    if (!isTauri()) return;
    getTestCriteria().then(setCriteria).catch(console.error);
  }, []);

  // Polling loop for GPS data and test status
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

        const isConnected = status.status === 'receiving_data' || status.status === 'connected';
        setConnected(isConnected);

        // Poll test status if a test is running
        if (testResult?.verdict === 'running' || testResult?.verdict === 'not_started') {
          const result = await getTestStatus();
          setTestResult(result);

          // If test just finished, add to history
          if (result.verdict === 'pass' || result.verdict === 'fail' || result.verdict === 'timed_out') {
            if (testResult?.verdict === 'running') {
              setTestHistory(prev => [...prev, result]);
            }
          }
        }
      } catch (error) {
        console.debug('Poll error:', error);
      }
    };

    poll();
    const interval = setInterval(poll, 500);
    return () => clearInterval(interval);
  }, [testResult?.verdict]);

  const handleConnected = useCallback(() => {
    setConnected(true);
  }, []);

  const handleReset = useCallback(async () => {
    // Disconnect, reset state, trigger re-detect
    try {
      await disconnectGps();
    } catch {
      // ignore
    }
    setGpsData(null);
    setGpsStatus(null);
    setTestResult(null);
    setConnected(false);
  }, []);

  const handleCriteriaChanged = useCallback((c: TestCriteria) => {
    setCriteria(c);
  }, []);

  return (
    <div className="app">
      {/* Header */}
      <header className="app-header">
        <div className="header-left">
          <h1>Scout GPS Test</h1>
          <span className="version">v1.0.0</span>
        </div>
        <div className="header-right">
          <button
            className={`btn btn-small ${showNmea ? 'btn-active' : ''}`}
            onClick={() => setShowNmea(!showNmea)}
          >
            NMEA
          </button>
          <button className="btn btn-small" onClick={() => setShowConfig(true)}>
            Config
          </button>
        </div>
      </header>

      {/* Main content - 3 zone layout */}
      <main className="app-main">
        <div className="zones">
          <DeviceDetection status={gpsStatus} onConnected={handleConnected} />
          <LiveDashboard gpsData={gpsData} criteria={criteria} />
          <TestRunner testResult={testResult} status={gpsStatus} onReset={handleReset} />
        </div>

        {/* Collapsible sections */}
        <NmeaTraffic visible={showNmea} />
        <TestHistory results={testHistory} />
      </main>

      {/* Config modal */}
      <ConfigPanel
        visible={showConfig}
        onClose={() => setShowConfig(false)}
        onCriteriaChanged={handleCriteriaChanged}
      />
    </div>
  );
}

export default App;

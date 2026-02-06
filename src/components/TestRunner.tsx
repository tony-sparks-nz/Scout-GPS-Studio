import { useState, useCallback } from 'react';
import {
  startTest,
  abortTest,
  saveTestReport,
  type TestResult,
  type GpsSourceStatus,
} from '../hooks/useTauri';

interface TestRunnerProps {
  testResult: TestResult | null;
  status: GpsSourceStatus | null;
  onReset: () => void;
}

export function TestRunner({ testResult, status, onReset }: TestRunnerProps) {
  const [saving, setSaving] = useState(false);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isConnected = status?.status === 'receiving_data' || status?.status === 'connected';
  const verdict = testResult?.verdict || 'not_started';
  const isRunning = verdict === 'running';
  const isDone = verdict === 'pass' || verdict === 'fail' || verdict === 'timed_out';

  const handleStart = useCallback(async () => {
    setError(null);
    setSavedPath(null);
    try {
      await startTest();
    } catch (e: any) {
      setError(e.message);
    }
  }, []);

  const handleAbort = useCallback(async () => {
    try {
      await abortTest();
    } catch (e: any) {
      setError(e.message);
    }
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const path = await saveTestReport();
      setSavedPath(path);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }, []);

  const handleNextTablet = useCallback(() => {
    setSavedPath(null);
    setError(null);
    onReset();
  }, [onReset]);

  return (
    <section className="panel test-runner">
      <h2>3. Test Results</h2>

      {/* Verdict banner */}
      {isDone && (
        <div className={`verdict-banner verdict-${verdict}`}>
          {verdict === 'pass' ? 'PASS' : verdict === 'timed_out' ? 'TIMED OUT' : 'FAIL'}
        </div>
      )}

      {/* Action buttons */}
      <div className="test-actions">
        {!isRunning && !isDone && (
          <button
            className="btn btn-large btn-start"
            onClick={handleStart}
            disabled={!isConnected}
          >
            {isConnected ? 'START TEST' : 'Connect GPS First'}
          </button>
        )}
        {isRunning && (
          <button className="btn btn-large btn-abort" onClick={handleAbort}>
            ABORT TEST
          </button>
        )}
        {isDone && (
          <div className="done-actions">
            <button
              className="btn btn-primary"
              onClick={handleSave}
              disabled={saving || !!savedPath}
            >
              {savedPath ? 'Saved' : saving ? 'Saving...' : 'Save Report'}
            </button>
            <button className="btn btn-start" onClick={handleNextTablet}>
              Next Tablet
            </button>
          </div>
        )}
      </div>

      {/* Elapsed time and TTFF */}
      {(isRunning || isDone) && testResult && (
        <div className="test-timing">
          <span>
            Elapsed: {testResult.test_duration_seconds.toFixed(1)}s
          </span>
          {testResult.ttff_seconds != null && (
            <span>TTFF: {testResult.ttff_seconds.toFixed(1)}s</span>
          )}
        </div>
      )}

      {/* Criteria table */}
      {testResult && testResult.criteria_results.length > 0 && (
        <table className="criteria-table">
          <thead>
            <tr>
              <th>Criterion</th>
              <th>Expected</th>
              <th>Actual</th>
              <th>Result</th>
            </tr>
          </thead>
          <tbody>
            {testResult.criteria_results.map((cr) => (
              <tr key={cr.name} className={cr.passed ? 'row-pass' : 'row-fail'}>
                <td>{cr.name}</td>
                <td>{cr.expected}</td>
                <td>{cr.actual}</td>
                <td className={cr.passed ? 'cell-pass' : 'cell-fail'}>
                  {cr.passed ? 'PASS' : 'FAIL'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {/* Messages */}
      {error && <div className="error-msg">{error}</div>}
      {savedPath && <div className="success-msg">Report saved: {savedPath}</div>}
    </section>
  );
}

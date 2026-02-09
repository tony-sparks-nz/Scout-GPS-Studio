// Tauri command bindings for GPS Studio

import { invoke } from '@tauri-apps/api/core';

// ============ Response Types ============

interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

// ============ GPS Types ============

export interface SatelliteInfo {
  prn: number;
  elevation: number | null;
  azimuth: number | null;
  snr: number | null;
  constellation: string;
}

export interface GpsData {
  latitude: number | null;
  longitude: number | null;
  speed_knots: number | null;
  course: number | null;
  heading: number | null;
  altitude: number | null;
  fix_quality: number | null;
  satellites: number | null;
  hdop: number | null;
  vdop: number | null;
  pdop: number | null;
  timestamp: string | null;
  fix_type: string | null;
  satellites_info: SatelliteInfo[];
}

export type GpsConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'receiving_data'
  | 'error';

export interface DetectedPort {
  port_name: string;
  port_type: string;
  manufacturer: string | null;
  product: string | null;
  serial_number: string | null;
  vid: number | null;
  pid: number | null;
  is_likely_gps: boolean;
}

export interface GpsSourceStatus {
  port_name: string | null;
  status: GpsConnectionStatus;
  last_error: string | null;
  sentences_received: number;
  last_fix_time: string | null;
}

// ============ Test Types ============

export interface TestCriteria {
  min_satellites: number;
  max_hdop: number;
  max_pdop: number;
  min_avg_snr: number;
  min_strong_satellites: number;
  max_ttff_seconds: number;
  min_constellations: number;
  min_fix_quality: number;
  stability_duration_seconds: number;
}

export interface CriterionResult {
  name: string;
  passed: boolean;
  expected: string;
  actual: string;
}

export type TestVerdict = 'pass' | 'fail' | 'running' | 'not_started' | 'timed_out';

export interface DeviceInfo {
  port_name: string;
  port_type: string;
  manufacturer: string | null;
  product: string | null;
  serial_number: string | null;
  vid: number | null;
  pid: number | null;
}

export interface TestResult {
  verdict: TestVerdict;
  criteria_results: CriterionResult[];
  ttff_seconds: number | null;
  test_duration_seconds: number;
  device_info: DeviceInfo;
  timestamp: string;
  best_gps_data: GpsData | null;
}

// ============ Utility ============

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// ============ GPS Commands ============

export async function listSerialPorts(): Promise<DetectedPort[]> {
  const result = await invoke<CommandResult<DetectedPort[]>>('list_serial_ports');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to list serial ports');
  }
  return result.data;
}

export async function autoDetectGps(): Promise<[DetectedPort, number]> {
  const result = await invoke<CommandResult<[DetectedPort, number]>>('auto_detect_gps');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to auto-detect GPS');
  }
  return result.data;
}

export async function testGpsPort(portName: string, baudRate: number): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('test_gps_port', { portName, baudRate });
  if (!result.success) {
    throw new Error(result.error || 'Failed to test GPS port');
  }
  return result.data ?? false;
}

export async function connectGps(portName: string, baudRate: number): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('connect_gps', { portName, baudRate });
  if (!result.success) {
    throw new Error(result.error || 'Failed to connect GPS');
  }
}

export async function disconnectGps(): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('disconnect_gps');
  if (!result.success) {
    throw new Error(result.error || 'Failed to disconnect GPS');
  }
}

export async function getGpsData(): Promise<GpsData> {
  const result = await invoke<CommandResult<GpsData>>('get_gps_data');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get GPS data');
  }
  return result.data;
}

export async function getGpsStatus(): Promise<GpsSourceStatus> {
  const result = await invoke<CommandResult<GpsSourceStatus>>('get_gps_status');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get GPS status');
  }
  return result.data;
}

export async function getNmeaBuffer(): Promise<string[]> {
  const result = await invoke<CommandResult<string[]>>('get_nmea_buffer');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get NMEA buffer');
  }
  return result.data;
}

export async function clearNmeaBuffer(): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('clear_nmea_buffer');
  if (!result.success) {
    throw new Error(result.error || 'Failed to clear NMEA buffer');
  }
}

// ============ Test Criteria Commands ============

export async function getTestCriteria(): Promise<TestCriteria> {
  const result = await invoke<CommandResult<TestCriteria>>('get_test_criteria');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get test criteria');
  }
  return result.data;
}

export async function setTestCriteria(criteria: TestCriteria): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('set_test_criteria', { criteria });
  if (!result.success) {
    throw new Error(result.error || 'Failed to set test criteria');
  }
}

export async function resetTestCriteria(): Promise<TestCriteria> {
  const result = await invoke<CommandResult<TestCriteria>>('reset_test_criteria');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to reset test criteria');
  }
  return result.data;
}

// ============ Test Execution Commands ============

export async function startTest(): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('start_test');
  if (!result.success) {
    throw new Error(result.error || 'Failed to start test');
  }
}

export async function getTestStatus(): Promise<TestResult> {
  const result = await invoke<CommandResult<TestResult>>('get_test_status');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get test status');
  }
  return result.data;
}

export async function abortTest(): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('abort_test');
  if (!result.success) {
    throw new Error(result.error || 'Failed to abort test');
  }
}

export async function saveTestReport(): Promise<string> {
  const result = await invoke<CommandResult<string>>('save_test_report');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to save test report');
  }
  return result.data;
}

export async function getRecentResults(): Promise<TestResult[]> {
  const result = await invoke<CommandResult<TestResult[]>>('get_recent_results');
  if (!result.success || !result.data) {
    throw new Error(result.error || 'Failed to get recent results');
  }
  return result.data;
}

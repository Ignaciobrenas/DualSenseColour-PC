import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EcosystemPanel } from '../components/EcosystemPanel';
import { DEFAULT_CONFIG, DEFAULT_STATUS } from '../types/controller';

const mockClearError = vi.fn();
const mockDismissLowBatteryAlert = vi.fn();

const mockHookState = {
  status: {
    ...DEFAULT_STATUS,
    connected: true,
    battery_percent: 10,
    charging_state: 'discharging' as const,
  },
  config: {
    ...DEFAULT_CONFIG,
    low_battery_alerts: true,
  },
  presets: [],
  error: null as string | null,
  lowBatteryAlert: null as { title: string; body: string; threshold: number } | null,
  autostartEnabled: false,
  clearError: mockClearError,
  dismissLowBatteryAlert: mockDismissLowBatteryAlert,
  updateColor: vi.fn(),
  applyPreset: vi.fn(),
  turnOffLightbar: vi.fn(),
  updatePlayerAndMic: vi.fn(),
  updatePreferences: vi.fn(),
  updateEffectMode: vi.fn(),
  updateAdaptiveTriggers: vi.fn(),
  exportProfileJson: vi.fn(),
  importProfileJson: vi.fn(),
  installCommunityProfile: vi.fn(),
  testOpenRgbSync: vi.fn(),
  testWledSync: vi.fn(),
  toggleAutostart: vi.fn(),
  selectController: vi.fn(),
  resyncController: vi.fn(),
  resetToDefaults: vi.fn(),
};

vi.mock('../hooks/useController', () => ({
  useController: () => mockHookState,
}));

import App from '../App';

describe('SenseHue notifications (X close & 5s auto-dismiss)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockClearError.mockReset();
    mockDismissLowBatteryAlert.mockReset();
    mockHookState.error = null;
    mockHookState.lowBatteryAlert = null;
    mockHookState.status.battery_percent = 10;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('closes low-battery banner with X button', () => {
    render(<App />);
    expect(screen.getByTestId('low-battery-banner')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('low-battery-close-btn'));
    expect(mockDismissLowBatteryAlert).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId('low-battery-banner')).not.toBeInTheDocument();
  });

  it('auto-dismisses low-battery banner after 5 seconds', () => {
    render(<App />);
    expect(screen.getByTestId('low-battery-banner')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(4999);
    });
    expect(screen.getByTestId('low-battery-banner')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(mockDismissLowBatteryAlert).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId('low-battery-banner')).not.toBeInTheDocument();
  });

  it('auto-dismisses friendly error toast after 5 seconds and allows X close', () => {
    mockHookState.error = 'Access denied opening HID interface';
    const { unmount } = render(<App />);

    expect(screen.getByTestId('friendly-error-toast')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('friendly-error-close-btn'));
    expect(mockClearError).toHaveBeenCalledTimes(1);

    unmount();
    mockClearError.mockReset();
    render(<App />);

    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(mockClearError).toHaveBeenCalledTimes(1);
  });

  it('closes EcosystemPanel bridge feedback with X or after 5 seconds', async () => {
    render(
      <EcosystemPanel
        config={DEFAULT_CONFIG}
        autostartEnabled={false}
        onPreferencesChange={vi.fn()}
        onToggleAutostart={vi.fn()}
        onTestOpenRgb={async () => 'OpenRGB conectado OK'}
        onTestWled={async () => 'WLED OK'}
      />
    );

    await act(async () => {
      fireEvent.click(screen.getByTestId('test-openrgb-btn'));
      await Promise.resolve();
    });
    expect(screen.getByTestId('bridge-feedback')).toHaveTextContent('OpenRGB conectado OK');

    fireEvent.click(screen.getByTestId('bridge-feedback-close-btn'));
    expect(screen.queryByTestId('bridge-feedback')).not.toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByTestId('test-wled-btn'));
      await Promise.resolve();
    });
    expect(screen.getByTestId('bridge-feedback')).toHaveTextContent('WLED OK');

    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(screen.queryByTestId('bridge-feedback')).not.toBeInTheDocument();
  });
});

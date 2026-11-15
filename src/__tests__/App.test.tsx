import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import App from '../App';
import { ConnectionBadge } from '../components/ConnectionBadge';
import { ControllerSwitcher } from '../components/ControllerSwitcher';
import { DEFAULT_CONFIG, formatFriendlyError } from '../types/controller';

describe('App integration', () => {
  it('syncs preset clicks, custom HEX edits, and LED power-off with DualSensePreview', async () => {
    render(<App />);

    expect(screen.getByText('SenseHue')).toBeInTheDocument();
    expect(screen.getByTestId('connection-badge')).toHaveTextContent('Sin mando');

    // Selecting Cyber Cyan updates live DualSense preview to #00F0FF
    fireEvent.click(screen.getByTestId('preset-cyber-cyan'));
    const preview = screen.getByTestId('dualsense-preview');
    expect(preview.getAttribute('data-color')).toBe('#00F0FF');

    // Entering valid custom HEX updates controller lightbar color
    fireEvent.change(screen.getByTestId('hex-input'), { target: { value: '#9D4EDD' } });
    expect(preview.getAttribute('data-color')).toBe('#9D4EDD');

    // Power-off button turns off lightbar cleanly
    fireEvent.click(screen.getByTestId('turn-off-btn'));
    expect(preview.getAttribute('data-brightness')).toBe('0');
    expect(screen.getByTestId('lightbar-overlay').getAttribute('opacity')).toBe('0');

    // Terms & Conditions modal opens from titlebar button and closes cleanly when accepted
    fireEvent.click(screen.getByTestId('open-terms-btn'));
    expect(screen.getByTestId('terms-modal')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('terms-accept-btn'));
    expect(screen.queryByTestId('terms-modal')).not.toBeInTheDocument();

    // Selecting a dynamic lighting effect mode updates active pressed state
    const spectrumBtn = screen.getByTestId('effect-mode-spectrum');
    fireEvent.click(spectrumBtn);
    expect(spectrumBtn).toHaveAttribute('aria-pressed', 'true');

    // Switching to Diagnóstico & Pro reveals telemetry, adaptive triggers, and Pro switches
    fireEvent.click(screen.getByTestId('tab-diagnostics'));
    expect(screen.getByTestId('diagnostics-panel')).toBeInTheDocument();

    const r2WeaponBtn = screen.getByTestId('trigger-right-weapon_click');
    fireEvent.click(r2WeaponBtn);
    expect(r2WeaponBtn).toHaveAttribute('aria-pressed', 'true');

    const batteryBarBtn = screen.getByTestId('toggle-battery-bar');
    fireEvent.click(batteryBarBtn);
    expect(batteryBarBtn).toHaveAttribute('aria-pressed', 'true');

    // New v3.0 competitive triggers & Gyro-Aim indicator work interactively
    const hairTriggerLeftBtn = screen.getByTestId('trigger-left-hair_trigger');
    fireEvent.click(hairTriggerLeftBtn);
    expect(hairTriggerLeftBtn).toHaveAttribute('aria-pressed', 'true');

    const gyroBtn = screen.getByTestId('toggle-gyro-indicator');
    fireEvent.click(gyroBtn);
    expect(gyroBtn).toHaveAttribute('aria-pressed', 'true');

    // Ecosistema tab exposes Zero-Injection Telemetry, OpenRGB, WLED & low-battery alerts switch
    fireEvent.click(screen.getByTestId('tab-ecosystem'));
    expect(screen.getByTestId('ecosystem-panel')).toBeInTheDocument();
    const gamesToggle = screen.getByTestId('toggle-game-integrations');
    fireEvent.click(gamesToggle);
    expect(gamesToggle).toHaveAttribute('aria-pressed', 'true');
    const alertsToggle = screen.getByTestId('toggle-low-battery-alerts');
    expect(alertsToggle).toHaveAttribute('aria-pressed', 'true');

    // Discover tab filters and applies curated community .sensehue profile onto preview
    fireEvent.click(screen.getByTestId('tab-discover'));
    expect(screen.getByTestId('discover-panel')).toBeInTheDocument();
    const applyF1Btn = screen.getByTestId('discover-apply-simracing-f1');
    fireEvent.click(applyF1Btn);
    await waitFor(() => {
      expect(preview.getAttribute('data-color')).toBe('#FF1744');
    });
  });

  it('renders battery readout in ConnectionBadge for Bluetooth and pending states', () => {
    const { rerender } = render(
      <ConnectionBadge
        status={{
          connected: true,
          transport: 'bluetooth',
          model: 'DualSense',
          serial_number: null,
          product_name: 'DualSense Wireless Controller',
          battery_percent: 80,
          charging_state: 'charging',
        }}
      />
    );

    expect(screen.getByTestId('battery-badge')).toHaveTextContent('80%');
    expect(screen.getByTestId('connection-badge')).toHaveTextContent('Bluetooth');

    rerender(
      <ConnectionBadge
        status={{
          connected: true,
          transport: 'bluetooth',
          model: 'DualSense',
          serial_number: null,
          product_name: null,
          battery_percent: null,
          charging_state: null,
        }}
      />
    );
    expect(screen.getByTestId('battery-badge')).toHaveTextContent('...');
  });

  it('renders low-battery badge thresholds at 20%, 10% and 5%', () => {
    const { rerender } = render(
      <ConnectionBadge
        status={{
          connected: true,
          transport: 'bluetooth',
          model: 'DualSense',
          serial_number: null,
          product_name: 'DualSense Wireless Controller',
          battery_percent: 20,
          charging_state: 'discharging',
        }}
      />
    );
    expect(screen.getByTestId('battery-badge')).toHaveTextContent('20%');

    rerender(
      <ConnectionBadge
        status={{
          connected: true,
          transport: 'bluetooth',
          model: 'DualSense',
          serial_number: null,
          product_name: 'DualSense Wireless Controller',
          battery_percent: 10,
          charging_state: 'discharging',
        }}
      />
    );
    expect(screen.getByTestId('battery-badge')).toHaveTextContent('10%');

    rerender(
      <ConnectionBadge
        status={{
          connected: true,
          transport: 'bluetooth',
          model: 'DualSense',
          serial_number: null,
          product_name: 'DualSense Wireless Controller',
          battery_percent: 5,
          charging_state: 'discharging',
        }}
      />
    );
    expect(screen.getByTestId('battery-badge')).toHaveTextContent('5%');
  });

  it('translates raw Windows HID errors into friendly actionable Spanish guidance', () => {
    const disconnected = formatFriendlyError('hid_error: Device not connected (0x0000048F)');
    expect(disconnected.title).toBe('Mando no detectado');
    expect(disconnected.detail).toContain('Reintentar');

    const blocked = formatFriendlyError('Access denied opening HID interface');
    expect(blocked.title).toBe('Acceso bloqueado al mando');
    expect(blocked.detail).toContain('Steam Input');
  });

  it('switches between broadcast sync and individual USB/Bluetooth controllers', () => {
    const onSelect = vi.fn();
    render(
      <ControllerSwitcher
        config={{
          ...DEFAULT_CONFIG,
          sync_all_controllers: true,
        }}
        status={{
          connected: true,
          transport: 'usb',
          model: 'DualSense',
          serial_number: 'SN-1',
          product_name: 'DualSense Wireless Controller',
          battery_percent: 80,
          charging_state: 'charging',
          active_controller_id: 'serial:SN-1',
          controllers: [
            {
              id: 'serial:SN-1',
              slot: 1,
              path: 'hid://usb/1',
              transport: 'usb',
              model: 'DualSense',
              serial_number: 'SN-1',
              product_name: 'DualSense USB',
              battery_percent: 80,
              charging_state: 'charging',
            },
            {
              id: 'serial:SN-2',
              slot: 2,
              path: 'hid://bt/2',
              transport: 'bluetooth',
              model: 'DualSenseEdge',
              serial_number: 'SN-2',
              product_name: 'DualSense Edge BT',
              battery_percent: 45,
              charging_state: 'discharging',
            },
          ],
        }}
        onSelect={onSelect}
      />
    );

    expect(screen.getByTestId('controller-switcher')).toBeInTheDocument();
    expect(screen.getByTestId('controller-sync-all-btn')).toHaveTextContent('Todos (2)');
    expect(screen.getByTestId('controller-slot-1')).toHaveTextContent('#1');
    expect(screen.getByTestId('controller-slot-1')).toHaveTextContent('80%');
    expect(screen.getByTestId('controller-slot-2')).toHaveTextContent('#2');
    expect(screen.getByTestId('controller-slot-2')).toHaveTextContent('45%');

    fireEvent.click(screen.getByTestId('controller-slot-2'));
    expect(onSelect).toHaveBeenCalledWith('serial:SN-2', false);

    fireEvent.click(screen.getByTestId('controller-sync-all-btn'));
    expect(onSelect).toHaveBeenCalledWith('serial:SN-1', true);
  });

  it('cycles all 7 L2/R2 adaptive trigger presets and resets back to Off cleanly', () => {
    render(<App />);

    fireEvent.click(screen.getByTestId('preset-cyber-cyan'));
    fireEvent.click(screen.getByTestId('tab-diagnostics'));

    const presets = [
      'rigid',
      'weapon_click',
      'pulse',
      'hair_trigger',
      'dual_stage',
      'progressive_pedal',
      'off',
    ] as const;

    for (const id of presets) {
      fireEvent.click(screen.getByTestId(`trigger-left-${id}`));
      expect(screen.getByTestId(`trigger-left-${id}`)).toHaveAttribute('aria-pressed', 'true');

      fireEvent.click(screen.getByTestId(`trigger-right-${id}`));
      expect(screen.getByTestId(`trigger-right-${id}`)).toHaveAttribute('aria-pressed', 'true');
    }

    // Toggling preferences must keep active custom color and trigger choices intact
    fireEvent.click(screen.getByTestId('trigger-left-pulse'));
    fireEvent.click(screen.getByTestId('toggle-battery-bar'));
    expect(screen.getByTestId('trigger-left-pulse')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByTestId('dualsense-preview').getAttribute('data-color')).toBe('#00F0FF');
  });
});

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import App from '../App';
import { ConnectionBadge } from '../components/ConnectionBadge';
import { formatFriendlyError } from '../types/controller';

describe('App integration', () => {
  it('syncs preset clicks, custom HEX edits, and LED power-off with DualSensePreview', () => {
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

  it('translates raw Windows HID errors into friendly actionable Spanish guidance', () => {
    const disconnected = formatFriendlyError('hid_error: Device not connected (0x0000048F)');
    expect(disconnected.title).toBe('Mando no detectado');
    expect(disconnected.detail).toContain('Reintentar');

    const blocked = formatFriendlyError('Access denied opening HID interface');
    expect(blocked.title).toBe('Acceso bloqueado al mando');
    expect(blocked.detail).toContain('Steam Input');
  });
});

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DualSensePreview } from '../components/DualSensePreview';

describe('DualSensePreview', () => {
  it('renders live RGB/HEX color and scaled brightness on the SVG lightbar', () => {
    render(<DualSensePreview r={0} g={240} b={255} brightness={80} playerLeds={3} />);

    const figure = screen.getByTestId('dualsense-preview');
    expect(figure.getAttribute('data-color')).toBe('#00F0FF');
    expect(figure.getAttribute('data-brightness')).toBe('80');

    const overlay = screen.getByTestId('lightbar-overlay');
    expect(overlay.getAttribute('opacity')).toBe('0.8');

    const dots = screen
      .getByTestId('player-leds-overlay')
      .querySelectorAll('circle[data-active="true"]');
    expect(dots).toHaveLength(3);
  });

  it('turns off lightbar glow cleanly when brightness is 0', () => {
    render(<DualSensePreview r={255} g={23} b={68} brightness={0} playerLeds={0} micLed="off" />);

    const overlay = screen.getByTestId('lightbar-overlay');
    expect(overlay.getAttribute('opacity')).toBe('0');

    const halo = screen.getByTestId('dualsense-ambient-halo');
    expect(halo.style.opacity).toBe('0');

    expect(screen.queryByTestId('mic-led-overlay')).toBeNull();
  });

  it('displays pulsing orange microphone mute LED when micLed is pulse', () => {
    render(<DualSensePreview r={0} g={67} b={156} brightness={100} micLed="pulse" />);

    const mic = screen.getByTestId('mic-led-overlay');
    expect(mic.getAttribute('class')).toContain('animate-mic-pulse');
  });
});

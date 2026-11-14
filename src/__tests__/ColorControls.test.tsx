import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ColorControls } from '../components/ColorControls';
import { BUILTIN_PRESETS, DEFAULT_CONFIG } from '../types/controller';

describe('ColorControls', () => {
  it('invokes callbacks for presets, HEX input, brightness, player LEDs and turn off', () => {
    const onColorChange = vi.fn();
    const onSelectPreset = vi.fn();
    const onTurnOff = vi.fn();
    const onPlayerAndMicChange = vi.fn();
    const onPreferencesChange = vi.fn();

    render(
      <ColorControls
        config={DEFAULT_CONFIG}
        presets={BUILTIN_PRESETS}
        onColorChange={onColorChange}
        onSelectPreset={onSelectPreset}
        onTurnOff={onTurnOff}
        onPlayerAndMicChange={onPlayerAndMicChange}
        onPreferencesChange={onPreferencesChange}
      />,
    );

    fireEvent.click(screen.getByTestId('preset-cyber-cyan'));
    expect(onSelectPreset).toHaveBeenCalledWith('cyber-cyan');

    const hexInput = screen.getByTestId('hex-input');
    fireEvent.change(hexInput, { target: { value: '#FF1744' } });
    expect(onColorChange).toHaveBeenCalledWith({ r: 255, g: 23, b: 68 }, 100, true);

    const brightness = screen.getByTestId('brightness-slider');
    fireEvent.change(brightness, { target: { value: '45' } });
    expect(onColorChange).toHaveBeenCalledWith(DEFAULT_CONFIG.color, 45, false);

    fireEvent.click(screen.getByTestId('player-led-4'));
    expect(onPlayerAndMicChange).toHaveBeenCalledWith(4, DEFAULT_CONFIG.mic_led);

    fireEvent.click(screen.getByTestId('mic-led-pulse'));
    expect(onPlayerAndMicChange).toHaveBeenCalledWith(DEFAULT_CONFIG.player_leds, 'pulse');

    fireEvent.click(screen.getByTestId('turn-off-btn'));
    expect(onTurnOff).toHaveBeenCalledTimes(1);
  });
});

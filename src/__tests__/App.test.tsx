import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import App from '../App';

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
  });
});

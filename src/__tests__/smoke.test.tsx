import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import App from '../App';

describe('App smoke test', () => {
  it('renders application title', () => {
    render(<App />);
    expect(screen.getByText('SenseHue')).toBeInTheDocument();
  });
});

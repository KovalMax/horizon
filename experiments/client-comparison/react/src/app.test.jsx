import React from 'react';
import { afterEach, expect, test } from 'vitest';
import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { App } from './main.jsx';

// jsdom does not implement the native dialog API; browser checks are still needed.
HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
afterEach(cleanup);

test('a member can create and complete a task', async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole('button', { name: 'Our household' }));
  await user.click(screen.getByRole('button', { name: '+ Add task' }));
  const dialog = screen.getByRole('dialog');
  await user.type(within(dialog).getByLabelText('Task name'), 'Buy tea');
  await user.click(within(dialog).getByRole('button', { name: 'Add task' }));
  await user.click(screen.getByRole('checkbox', { name: /Buy tea/ }));
  expect(screen.getByRole('checkbox', { name: /Buy tea/ }).checked).toBe(true);
});

test('a member can open and close People', async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole('button', { name: 'Our household' }));
  await user.click(screen.getByRole('button', { name: 'People · 2' }));
  expect(within(screen.getByRole('dialog')).getByText('Jamie')).toBeTruthy();
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('a member can expand and collapse the calendar', async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole('button', { name: 'Our household' }));
  await user.click(screen.getByRole('button', { name: 'Expand calendar ↗' }));
  expect(screen.getByText('June 2026')).toBeTruthy();
  await user.click(screen.getByRole('button', { name: 'Collapse ↑' }));
  expect(screen.queryByText('June 2026')).toBeNull();
});

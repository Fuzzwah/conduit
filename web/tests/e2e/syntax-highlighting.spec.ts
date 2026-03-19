import { expect, test } from '@playwright/test';
import { mockApi } from './fixtures';
import { installMockWebSocket } from './websocket-mock';

const syntaxEventsResponse = {
  events: [
    { role: 'user', content: 'Show me code.' },
    {
      role: 'assistant',
      content: [
        'Use `cargo check --all` before pushing.',
        '',
        '```rust',
        'fn main() {',
        '    let answer = 42;',
        '    println!("{answer}");',
        '}',
        '```',
      ].join('\n'),
    },
  ],
  total: 2,
  offset: 0,
  limit: 200,
  debug_file: null,
  debug_entries: [],
};

test.beforeEach(async ({ page }) => {
  await mockApi(page, { sessionEvents: syntaxEventsResponse });
  await installMockWebSocket(page);
});

test('markdown code uses active theme syntax colors and inline code stays transparent', async ({
  page,
}) => {
  await page.goto('/');
  await page.waitForResponse('**/api/bootstrap');
  await expect(page.getByPlaceholder('Type a message...')).toBeVisible();

  const shikiBlock = page.locator('.shiki').first();
  await expect(shikiBlock).toBeVisible();

  const darkBlockHtml = await shikiBlock.evaluate((node) => node.innerHTML);
  expect(darkBlockHtml).toContain('#3B82F6');

  const inlineCode = page.locator('code', { hasText: 'cargo check --all' }).first();
  const inlineBackground = await inlineCode.evaluate(
    (node) => getComputedStyle(node).backgroundColor
  );
  expect(inlineBackground).toBe('rgba(0, 0, 0, 0)');

  await page.getByLabel('Select theme').click();
  await page.getByRole('option', { name: 'Default Light' }).click();

  await expect
    .poll(async () => shikiBlock.evaluate((node) => node.innerHTML))
    .toContain('#DC2626');

  const inlineColor = await inlineCode.evaluate((node) => getComputedStyle(node).color);
  expect(inlineColor).toBe('rgb(17, 24, 39)');
});

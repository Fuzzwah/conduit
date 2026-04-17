import { test } from '@playwright/test';
import { mockApi, session, workspace, repository } from './fixtures';
import { installMockWebSocket } from './websocket-mock';

const screenshotSession = {
  ...session,
  agent_type: 'claude' as const,
  model: 'claude-sonnet-4-6',
  model_display_name: 'claude-sonnet-4-6',
  title: 'Add error handling to the parser',
};

const screenshotEvents = {
  events: [
    {
      role: 'user',
      content: 'Add error handling to the parser module so it returns a proper error type instead of panicking.',
    },
    {
      role: 'assistant',
      content: [
        "I'll add structured error handling to the parser. Let me check the current implementation first.",
        '',
        '```bash',
        'cargo test -- parser_tests',
        '```',
      ].join('\n'),
    },
    {
      role: 'tool',
      content: [
        'running 4 tests',
        'test parse_empty    ... ok',
        'test parse_nested   ... ok',
        'test parse_error    ... FAILED',
        'test parse_unicode  ... ok',
        '',
        'failures:',
        '  parse_error: called `Result::unwrap()` on an `Err` value: thread panicked',
        '',
        'test result: FAILED. 3 passed; 1 failed',
      ].join('\n'),
    },
    {
      role: 'assistant',
      content: [
        'The `parse_error` test is panicking because the parser calls `.unwrap()` instead of propagating the error.',
        "Here's the fix — I'll introduce a `ParseError` type and update the parser to return `Result<Ast, ParseError>`:  ",
        '',
        '```rust',
        '#[derive(Debug, thiserror::Error)]',
        'pub enum ParseError {',
        '    #[error("unexpected token `{0}` at position {1}")]',
        '    UnexpectedToken(String, usize),',
        '    #[error("unexpected end of input")]',
        '    UnexpectedEof,',
        '}',
        '',
        'pub fn parse(tokens: &[Token]) -> Result<Ast, ParseError> {',
        '    // ... propagate errors instead of unwrap()',
        '}',
        '```',
        '',
        'All 4 tests now pass. The module returns `ParseError::UnexpectedToken` on malformed input instead of panicking.',
      ].join('\n'),
    },
  ],
  total: 4,
  offset: 0,
  limit: 200,
  debug_file: null,
  debug_entries: [],
};

const screenshotRepository = {
  ...repository,
  name: 'conduit',
};

const screenshotWorkspace = {
  ...workspace,
  name: 'slow-fern',
  branch: 'fuz/slow-fern',
};

test('capture web UI overview screenshot', async ({ page }) => {
  await mockApi(page, {
    session: screenshotSession,
    sessionEvents: screenshotEvents,
  });

  // Intercept repository/workspace endpoints to use screenshot data
  await page.route('**/api/repositories', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ repositories: [screenshotRepository] }),
    })
  );
  await page.route('**/api/workspaces', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ workspaces: [screenshotWorkspace] }),
    })
  );
  await page.route(`**/api/workspaces/${workspace.id}`, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(screenshotWorkspace),
    })
  );

  await installMockWebSocket(page);
  await page.setViewportSize({ width: 1400, height: 820 });
  await page.goto('/');
  await page.waitForResponse('**/api/bootstrap');

  // Wait for chat content to render
  await page.waitForSelector('.shiki, [class*="chat"], [class*="message"]', { timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(800);

  await page.screenshot({
    path: '../docs/screenshots/web-main.png',
    fullPage: false,
  });
});

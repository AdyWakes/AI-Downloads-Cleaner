// Captures every screen against the mock backend. Usage:
//   npm run build && npx vite preview --port 4173 &
//   node scripts/screenshots.cjs <out-dir> [deviceScaleFactor]
// Needs Playwright with Chromium available (not a project dependency).
const { chromium } = require('playwright');
(async () => {
  const out = process.argv[2];
  const scale = Number(process.argv[3] || 1);
  const b = await chromium.launch();
  for (const scheme of ['light', 'dark']) {
    const p = await b.newPage({ viewport: { width: 1100, height: 760 }, colorScheme: scheme, deviceScaleFactor: scale });
    const errs = [];
    p.on('pageerror', e => errs.push(e.message));
    p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
    const shot = (n, o = {}) => p.screenshot({ path: `${out}/${scheme}-${n}.png`, ...o });
    await p.goto('http://localhost:4173');
    await p.waitForSelector('text=Scan folder');
    await p.waitForTimeout(500);
    await shot('01-home');
    await p.click('text=Scan folder');
    await p.waitForTimeout(350);
    await shot('02-scanning');
    await p.waitForSelector('text=Review the plan', { timeout: 10000 });
    await p.waitForTimeout(300);
    await shot('03-stats');
    await shot('03-stats-full', { fullPage: true });
    await p.click('text=Review the plan');
    await p.waitForTimeout(300);
    await shot('04-preview');
    await p.hover('li:has-text("resume_final_v3.docx")');
    await p.waitForTimeout(200);
    await shot('04-preview-hover');
    await p.click('button:has-text("Clean up")');
    await p.waitForTimeout(250);
    await shot('05-cleaning');
    await p.waitForSelector('text=Undo this cleanup', { timeout: 10000 });
    await p.waitForTimeout(250);
    await shot('06a-done-before');
    for (const t of [400, 800, 1200, 1600]) { await p.waitForTimeout(400); await shot(`06b-done-${t}`); }
    await p.waitForTimeout(1800);
    await shot('06c-done-after');
    await p.click('text=Undo this cleanup');
    await p.waitForSelector('text=Everything is back');
    await p.waitForTimeout(400);
    await shot('07-undone');
    await p.click('button:has-text("Done")');
    await p.click('text=Scan folder');
    await p.waitForSelector('text=Review the plan', { timeout: 10000 });
    await p.click('text=Review the plan');
    await p.click('button:has-text("Clean up")');
    await p.waitForSelector('text=Undo this cleanup', { timeout: 10000 });
    await p.click('button:has-text("History")');
    await p.waitForTimeout(300);
    await shot('08-history');
    await p.click('button:has-text("Settings")');
    await p.waitForTimeout(300);
    await shot('09-settings');
    await p.click('button[role=tab]:has-text("Rules")');
    await p.waitForTimeout(200);
    await shot('10-rules');
    await p.click('button[role=tab]:has-text("AI")');
    await shot('11-settings-ai');
    await p.click('button:has-text("AI: on")');
    await p.waitForTimeout(400);
    await shot('12-ai-dialog');
    console.log(scheme, 'errors:', JSON.stringify(errs));
    await p.close();
  }
  await b.close();
})();

// Native regression: nsh test window-shortcuts.test.tsx --target macos
import { expect, open, test } from "nsh/test";
for (const focused of [false, true]) {
  test(`Command+W closes fixture with editor focused=${focused}`, async () => {
    const app = await open(".", { width: 900, height: 650, colorScheme: "light" });
    await app.press("label:Preview scene");
    await app.press("role:option[window-shortcuts]");
    await app.press("Open fixture");
    await app.clear("label:Fixture message");
    await app.type("label:Fixture message", "Fixture draft preserved");
    if (!focused) await app.press("Focus outside");
    await app.keyboard.press("mod+w");
    expect(app.window("close-fixture").state()).toBe(null);
    expect(app.window("main").state()?.visible).toBe(true);
    await app.press("Open fixture");
    expect(app.texts()).toContain("Fixture draft preserved");
  });
}

test("Command+W during IME composition retains the committed draft", async () => {
  const app = await open(".", { width: 900, height: 650 });
  await app.press("label:Preview scene");
  await app.press("role:option[window-shortcuts]");
  await app.press("Open fixture");
  await app.compose("label:Fixture message", "ni");
  await app.keyboard.press("mod+w");
  expect(app.window("close-fixture").state()).toBe(null);
  expect(app.window("main").state()?.visible).toBe(true);
  await app.press("Open fixture");
  expect(app.texts()).toContain("Fixture draft");
});

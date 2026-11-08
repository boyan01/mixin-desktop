// Frame-rate readings for the stress scenes; it asserts nothing. From nsh/preview:
//   CARGO_TARGET_DIR=../../target nsh test perf/stress.test.tsx --target macos --fps -v
// then read the `nsh::perf` lines between each PERF-MARK start/end.
import { open, test } from "nsh/test";

const runs: [string, string, number][] = [
  ["messages-stress", "Scroll", 4500],
  ["messages-stress", "JumpFar", 3000],
  ["messages-stress", "JumpNear", 3000],
  ["conversations-stress", "Scroll", 4500],
  ["conversations-stress", "JumpFar", 3000],
];

test("stress perf", async () => {
  const app = await open("..", { width: 900, height: 650, locale: "en", colorScheme: "light" });
  for (const [scene, action, wait] of runs) {
    await app.press(`label:Scene ${scene}`);
    await app.wait(1500);
    for (let round = 0; round < 3; round++) {
      console.log(`PERF-MARK ${scene} ${action} round ${round} start ${Date.now()}`);
      await app.press(`label:Action ${action}`);
      await app.wait(wait);
      console.log(`PERF-MARK ${scene} ${action} round ${round} end ${Date.now()}`);
    }
  }
}, 600_000);

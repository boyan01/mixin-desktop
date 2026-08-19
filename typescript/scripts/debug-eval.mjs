import { evaluate } from './ui-debug-client.mjs';

const expression = process.argv.slice(2).join(' ').trim();

if (!expression) {
  console.error("Usage: npm run debug:eval -- 'document.title'");
  process.exit(2);
}

try {
  const { response, result } = await evaluate(expression);
  console.log(JSON.stringify(result, null, 2));
  if (!response.ok || result.ok === false) process.exitCode = 1;
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

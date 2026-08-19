import { ImageOff } from 'lucide-react';
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type JSX } from 'react';
import { openExternal } from '@/shared/lib/openExternal';
import { classes } from '@/shared/styles/classes';

type MessageToken =
  | { kind: 'text'; value: string }
  | { kind: 'url'; value: string }
  | { kind: 'mail'; value: string }
  | { kind: 'mention'; value: string; identityNumber: string }
  | { kind: 'bot'; value: string; identityNumber: string };

const messageTokenPattern = /https?:\/\/[^\s<>()]+|mixin:\/\/[^\s<>()]+|[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}|@\d{4,}|(?<!\d)7000\d{6}(?!\d)/gi;
const trailingUrlPunctuation = /[.,!?;:]+$/;

export function tokenizeMessageText(content: string): MessageToken[] {
  const tokens: MessageToken[] = [];
  const push = (token: MessageToken) => {
    const previous = tokens.at(-1);
    if (token.kind === 'text' && previous?.kind === 'text') previous.value += token.value;
    else tokens.push(token);
  };
  let offset = 0;
  for (const match of content.matchAll(messageTokenPattern)) {
    const index = match.index ?? 0;
    if (index > offset) push({ kind: 'text', value: content.slice(offset, index) });
    const raw = match[0];
    const value = /^(?:https?:\/\/|mixin:\/\/)/i.test(raw) ? raw.replace(trailingUrlPunctuation, '') : raw;
    const remainder = raw.slice(value.length);
    if (/^@\d{4,}$/.test(value)) push({ kind: 'mention', value, identityNumber: value.slice(1) });
    else if (/^7000\d{6}$/.test(value)) push({ kind: 'bot', value, identityNumber: value });
    else if (/^(?:https?:\/\/|mixin:\/\/)/i.test(value)) push({ kind: 'url', value });
    else push({ kind: 'mail', value });
    if (remainder) push({ kind: 'text', value: remainder });
    offset = index + raw.length;
  }
  if (offset < content.length) push({ kind: 'text', value: content.slice(offset) });
  return tokens;
}

export function SelectableMessageText({
  content,
  mentionNames = {},
  onOpenIdentity,
  className = 'message-text',
}: {
  content: string;
  mentionNames?: Record<string, string>;
  onOpenIdentity?: (identityNumber: string) => void;
  className?: string;
}) {
  return <p className={classes(className)} onContextMenu={(event) => event.preventDefault()}>{tokenizeMessageText(content).map((token, index) => {
    if (token.kind === 'text') return token.value;
    if (token.kind === 'mention') return <button className={classes("message-inline-link message-mention")} type="button" key={index} onClick={() => onOpenIdentity?.(token.identityNumber)}>@{mentionNames[token.identityNumber] || token.identityNumber}</button>;
    if (token.kind === 'bot') return <button className={classes("message-inline-link")} type="button" key={index} onClick={() => onOpenIdentity?.(token.identityNumber)}>{token.value}</button>;
    if (token.kind === 'mail') return <a className={classes("message-inline-link")} key={index} href={`mailto:${token.value}`} onClick={(event) => { event.preventDefault(); openExternal(`mailto:${token.value}`); }}>{token.value}</a>;
    return <a className={classes("message-inline-link")} key={index} href={token.value} onClick={(event) => { event.preventDefault(); openExternal(token.value); }}>{token.value}</a>;
  })}</p>;
}

export function MoreExtendedText({ text, className = '' }: { text: string; className?: string }) {
  const [expanded, setExpanded] = useState(false);
  const [endIndex, setEndIndex] = useState<number>();
  const measureRef = useRef<HTMLParagraphElement>(null);
  const overflowRef = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    setExpanded(false);
  }, [text]);
  useLayoutEffect(() => {
    const element = measureRef.current;
    const overflow = overflowRef.current;
    if (!element || !overflow || expanded) return;
    const update = () => {
      const node = element.firstChild;
      if (!node || node.nodeType !== Node.TEXT_NODE || element.clientWidth === 0) return;
      const range = document.createRange();
      const rectsFor = (end: number) => {
        range.setStart(node, 0);
        range.setEnd(node, end);
        return [...range.getClientRects()].filter((rect) => rect.width > 0 && rect.height > 0);
      };
      const all = rectsFor(text.length);
      if (all.length <= 6) {
        setEndIndex(undefined);
        return;
      }
      // Flutter's TextPainter reserves the right edge of its sixth line for
      // `...More`, then takes the character just before that position.
      const reservedRight = element.getBoundingClientRect().right - overflow.getBoundingClientRect().width;
      let low = 0;
      let high = text.length;
      while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        const rects = rectsFor(middle);
        const last = rects.at(-1);
        const fits = Boolean(last && rects.length <= 6 && (rects.length < 6 || last.right <= reservedRight + .5));
        if (fits) low = middle;
        else high = middle - 1;
      }
      setEndIndex(low > 0 ? low : 0);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [expanded, text]);
  const truncated = !expanded && endIndex !== undefined;
  return <div className={classes(`more-extended-text ${expanded ? 'is-expanded' : ''} ${className}`.trim())}>
    <p className={classes("more-extended-text-measure")} ref={measureRef} aria-hidden="true">{text}</p>
    <p>{truncated ? text.slice(0, endIndex) : text}{truncated && <button type="button" onClick={() => setExpanded(true)}>...More</button>}</p>
    <span className={classes("more-extended-text-overflow-measure")} ref={overflowRef} aria-hidden="true">...More</span>
  </div>;
}

function inlineMarkdown(value: string): ReactNode[] {
  const result: ReactNode[] = [];
  const pattern = /!\[([^\]]*)\]\(([^\s)]+)(?:\s+[^)]*)?\)|\[([^\]]+)\]\(([^\s)]+)(?:\s+[^)]*)?\)|`([^`]+)`|\*\*([^*]+)\*\*|__([^_]+)__|\*([^*]+)\*|_([^_]+)_/g;
  let offset = 0;
  for (const match of value.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > offset) result.push(...plainInline(value.slice(offset, index), result.length));
    const [raw, imageAlt, imageUrl, linkLabel, linkUrl, code, boldStar, boldUnder, italicStar, italicUnder] = match;
    const key = `m-${index}`;
    if (imageUrl) result.push(<MarkdownImage key={key} url={imageUrl} alt={imageAlt || 'Image'} />);
    else if (linkUrl) result.push(<a className={classes("message-inline-link")} key={key} href={linkUrl} onClick={(event) => { event.preventDefault(); openExternal(linkUrl); }}>{linkLabel}</a>);
    else if (code) result.push(<code key={key}>{code}</code>);
    else if (boldStar || boldUnder) result.push(<strong key={key}>{boldStar || boldUnder}</strong>);
    else result.push(<em key={key}>{italicStar || italicUnder}</em>);
    offset = index + raw.length;
  }
  if (offset < value.length) result.push(...plainInline(value.slice(offset), result.length));
  return result;
}

/** Matches post_markdown.dart: fixed-size images retain their reserved box on failure. */
function MarkdownImage({ url, alt }: { url: string; alt: string }) {
  const [failed, setFailed] = useState(false);
  const style = markdownImageStyle(url);
  useEffect(() => setFailed(false), [url]);
  if (failed) {
    if (style) return <span className={classes("markdown-image-error markdown-image-error-sized")} style={style} role="img" aria-label={alt}><ImageOff size={24} /></span>;
    return <span className={classes("markdown-image-error markdown-image-error-inline")} role="img" aria-label={alt}><ImageOff size={18} /><span>{alt}</span></span>;
  }
  return <img className={classes("markdown-image")} src={url} style={style} alt={alt} onError={() => setFailed(true)} />;
}

function plainInline(value: string, offset: number): ReactNode[] {
  return tokenizeMessageText(value).map((token, index) => {
    const key = `p-${offset + index}`;
    if (token.kind === 'text') return token.value;
    if (token.kind === 'mail') return <a className={classes("message-inline-link")} key={key} href={`mailto:${token.value}`} onClick={(event) => { event.preventDefault(); openExternal(`mailto:${token.value}`); }}>{token.value}</a>;
    if (token.kind === 'url') return <a className={classes("message-inline-link")} key={key} href={token.value} onClick={(event) => { event.preventDefault(); openExternal(token.value); }}>{token.value}</a>;
    return token.value;
  });
}

export function MarkdownColumn({ data, className = '' }: { data: string; className?: string }) {
  const blocks: ReactNode[] = [];
  const lines = data.replace(/\r\n?/g, '\n').split('\n');
  for (let index = 0; index < lines.length;) {
    const line = lines[index];
    if (line.startsWith('```')) {
      const code: string[] = [];
      const language = line.slice(3).trim();
      index += 1;
      while (index < lines.length && !lines[index].startsWith('```')) code.push(lines[index++]);
      if (index < lines.length) index += 1;
      blocks.push(<pre key={`code-${index}`} data-language={language || undefined}><code>{code.join('\n')}</code></pre>);
      continue;
    }
    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const Tag = `h${level}` as keyof JSX.IntrinsicElements;
      blocks.push(<Tag key={`h-${index}`}>{inlineMarkdown(heading[2])}</Tag>);
      index += 1;
      continue;
    }
    if (/^\s*([-*_])(?:\s*\1){2,}\s*$/.test(line)) { blocks.push(<hr key={`hr-${index}`} />); index += 1; continue; }
    if (line.startsWith('> ')) {
      const quote: string[] = [];
      while (index < lines.length && lines[index].startsWith('> ')) quote.push(lines[index++].slice(2));
      blocks.push(<blockquote key={`q-${index}`}>{inlineMarkdown(quote.join('\n'))}</blockquote>);
      continue;
    }
    if (isTableSeparator(lines[index + 1])) {
      const header = markdownTableCells(line);
      index += 2;
      const rows: string[][] = [];
      while (index < lines.length && lines[index].includes('|') && lines[index].trim()) rows.push(markdownTableCells(lines[index++]));
      blocks.push(<div className={classes("markdown-table-scroll")} key={`table-${index}`}><table><thead><tr>{header.map((cell, cellIndex) => <th key={cellIndex}>{inlineMarkdown(cell)}</th>)}</tr></thead><tbody>{rows.map((row, rowIndex) => <tr key={rowIndex}>{header.map((_, cellIndex) => <td key={cellIndex}>{inlineMarkdown(row[cellIndex] || '')}</td>)}</tr>)}</tbody></table></div>);
      continue;
    }
    const unordered = /^[-*+]\s+(.+)$/.exec(line);
    const ordered = /^\d+[.)]\s+(.+)$/.exec(line);
    if (unordered || ordered) {
      const items: string[] = [];
      const matcher = ordered ? /^\d+[.)]\s+(.+)$/ : /^[-*+]\s+(.+)$/;
      while (index < lines.length) { const item = matcher.exec(lines[index]); if (!item) break; items.push(item[1]); index += 1; }
      const List = ordered ? 'ol' : 'ul';
      blocks.push(<List key={`list-${index}`}>{items.map((item, itemIndex) => {
        const task = markdownTask(item);
        return <li className={classes(task ? 'markdown-task-item' : undefined)} key={itemIndex}>{task ? <><input type="checkbox" checked={task.checked} readOnly tabIndex={-1} aria-label={task.checked ? 'Completed task' : 'Incomplete task'} />{inlineMarkdown(task.content)}</> : inlineMarkdown(item)}</li>;
      })}</List>);
      continue;
    }
    if (!line.trim()) { index += 1; continue; }
    const paragraph: string[] = [line];
    index += 1;
    while (index < lines.length && lines[index].trim() && !/^(#{1,6}\s+|```|> |[-*+]\s+|\d+[.)]\s+)/.test(lines[index])) paragraph.push(lines[index++]);
    blocks.push(<p key={`p-${index}`}>{inlineMarkdown(paragraph.join('\n'))}</p>);
  }
  return <div className={classes(`markdown-column ${className}`.trim())}>{blocks}</div>;
}

/** Splits a GFM table row while accepting both optional outer pipes. */
export function markdownTableCells(line: string) {
  const trimmed = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  return trimmed.split('|').map((cell) => cell.trim());
}

function isTableSeparator(line: string | undefined) {
  if (!line || !line.includes('|')) return false;
  return markdownTableCells(line).every((cell) => /^:?-{3,}:?$/.test(cell));
}

export function markdownTask(value: string): { checked: boolean; content: string } | undefined {
  const match = /^\[([ xX])\]\s+(.+)$/.exec(value);
  return match ? { checked: match[1].toLowerCase() === 'x', content: match[2] } : undefined;
}

export function markdownImageStyle(url: string): CSSProperties | undefined {
  const params = new URL(url, 'https://mixin.local').searchParams;
  const width = Number(params.get('w') || params.get('width'));
  const height = Number(params.get('h') || params.get('height'));
  return Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0 ? { width, height } : undefined;
}

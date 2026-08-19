const MASK_64 = (1n << 64n) - 1n;

const ROUND_CONSTANTS = [
  0x0000000000000001n, 0x0000000000008082n, 0x800000000000808an, 0x8000000080008000n,
  0x000000000000808bn, 0x0000000080000001n, 0x8000000080008081n, 0x8000000000008009n,
  0x000000000000008an, 0x0000000000000088n, 0x0000000080008009n, 0x000000008000000an,
  0x000000008000808bn, 0x800000000000008bn, 0x8000000000008089n, 0x8000000000008003n,
  0x8000000000008002n, 0x8000000000000080n, 0x000000000000800an, 0x800000008000000an,
  0x8000000080008081n, 0x8000000000008080n, 0x0000000080000001n, 0x8000000080008008n,
];

const ROTATIONS = [
  0, 1, 62, 28, 27,
  36, 44, 6, 55, 20,
  3, 10, 43, 25, 39,
  41, 45, 15, 21, 8,
  18, 2, 61, 56, 14,
];

function rotateLeft64(value: bigint, amount: number) {
  if (amount === 0) return value;
  return ((value << BigInt(amount)) | (value >> BigInt(64 - amount))) & MASK_64;
}

function keccakF1600(state: bigint[]) {
  for (const constant of ROUND_CONSTANTS) {
    const column = Array.from({ length: 5 }, (_, x) => state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20]);
    const delta = Array.from({ length: 5 }, (_, x) => column[(x + 4) % 5] ^ rotateLeft64(column[(x + 1) % 5], 1));
    for (let x = 0; x < 5; x += 1) {
      for (let y = 0; y < 5; y += 1) state[x + 5 * y] ^= delta[x];
    }
    const rotated = Array<bigint>(25);
    for (let x = 0; x < 5; x += 1) {
      for (let y = 0; y < 5; y += 1) rotated[y + 5 * ((2 * x + 3 * y) % 5)] = rotateLeft64(state[x + 5 * y], ROTATIONS[x + 5 * y]);
    }
    for (let x = 0; x < 5; x += 1) {
      for (let y = 0; y < 5; y += 1) state[x + 5 * y] = rotated[x + 5 * y] ^ ((~rotated[(x + 1) % 5 + 5 * y]) & rotated[(x + 2) % 5 + 5 * y]);
    }
    state[0] ^= constant;
  }
}

/** SHA3-256, matching PointyCastle's SHA3Digest(256) used by Flutter. */
export function sha3_256(input: Uint8Array) {
  const rate = 136;
  const paddedLength = Math.ceil((input.length + 1) / rate) * rate;
  const padded = new Uint8Array(paddedLength);
  padded.set(input);
  padded[input.length] = 0x06;
  padded[padded.length - 1] |= 0x80;
  const state = Array<bigint>(25).fill(0n);

  for (let offset = 0; offset < padded.length; offset += rate) {
    for (let lane = 0; lane < rate / 8; lane += 1) {
      let value = 0n;
      for (let byte = 0; byte < 8; byte += 1) value |= BigInt(padded[offset + lane * 8 + byte]) << BigInt(byte * 8);
      state[lane] ^= value;
    }
    keccakF1600(state);
  }

  const output = new Uint8Array(32);
  for (let index = 0; index < output.length; index += 1) output[index] = Number((state[Math.floor(index / 8)] >> BigInt((index % 8) * 8)) & 0xffn);
  return output;
}

export function inscriptionDisplayContent(content: string) {
  return content.replace(/[\s\p{Other}\p{Cf}\p{Cc}\p{Cn}]/gu, '■');
}

export function inscriptionColors(hash: string | null | undefined) {
  if (!hash || hash.length % 2 || !/^[\da-f]+$/i.test(hash)) return Array(12).fill('#0000001f');
  const bytes = Uint8Array.from(hash.match(/.{2}/g)!.map((byte) => Number.parseInt(byte, 16)));
  const data = [...bytes, ...sha3_256(bytes).subarray(0, 4)];
  return Array.from({ length: Math.floor(data.length / 3) }, (_, index) => {
    const offset = index * 3;
    return `rgb(${data[offset]}, ${data[offset + 1]}, ${data[offset + 2]})`;
  });
}

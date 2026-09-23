// Simuliert den Moon Lander deterministisch, ohne Zugriff auf Finanzdaten oder Plattformdienste.
export type Flight = { x: number; y: number; vx: number; vy: number; fuel: number; status: "ready" | "flying" | "paused" | "landed" | "crashed" };
export type Controls = { thrust: boolean; left: boolean; right: boolean };
export const PAD = { left: 590, right: 750, top: 360 };
export const GROUND = 384;
export const FEET = 28;
export const HALF_WIDTH = 26;
export const MAX_VX = 20;
export const MAX_VY = 30;
export function newFlight(): Flight {
  return { x: 620, y: 70, vx: -14, vy: 0, fuel: 100, status: "ready" };
}
export function stepFlight(previous: Flight, input: Controls, seconds: number): Flight {
  if (previous.status !== "flying" || !Number.isFinite(seconds) || seconds <= 0) return previous;
  const s = { ...previous };
  const dt = Math.min(seconds, 1 / 30);
  const direction = Number(input.right) - Number(input.left);
  const burn = (input.thrust ? 7 : 0) + (direction ? 3 : 0);
  const powered = burn ? Math.min(dt, s.fuel / burn) : 0;
  s.fuel = Math.max(0, s.fuel - burn * powered);
  s.vx += direction * 32 * powered;
  s.vy += 18 * dt - (input.thrust ? 46 * powered : 0);
  s.x += s.vx * dt;
  s.y += s.vy * dt;
  const overPad = s.x + HALF_WIDTH >= PAD.left && s.x - HALF_WIDTH <= PAD.right;
  const floor = overPad ? PAD.top : GROUND;
  if (s.x < HALF_WIDTH || s.x > 900 - HALF_WIDTH || s.y < 24) s.status = "crashed";
  if (s.y + FEET >= floor) {
    s.y = floor - FEET;
    s.status = previous.y + FEET <= PAD.top && s.x - HALF_WIDTH >= PAD.left && s.x + HALF_WIDTH <= PAD.right &&
      Math.abs(s.vx) <= MAX_VX && Math.abs(s.vy) <= MAX_VY ? "landed" : "crashed";
  }
  return s;
}

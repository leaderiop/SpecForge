/** Adds two numbers. */
export function add(a: number, b: number): number {
  return a + b;
}

export function parse(text: string): number;
export function parse(text: number): number;
export function parse(text: string | number): number {
  return Number(text);
}

export class Board {
  private cells: number[] = [];
  static empty(): Board {
    return new Board();
  }
  place(cell: number): void {
    this.cells.push(cell);
  }
}

export interface Shape {
  area(): number;
}

export type Id = string;
export const Id = "id";

export namespace Geo {
  export function distance(a: number, b: number): number {
    return Math.abs(a - b);
  }
}

function helper(): void {}

/** A widget. */
export declare class Widget {
    size: number;
    render(): string;
}
export interface Options {
    mode: Mode;
}
export declare enum Mode {
    Fast = 0,
    Slow = 1
}
export declare function create(o: Options): Widget;
export type Id = string;
export declare const VERSION = "1";

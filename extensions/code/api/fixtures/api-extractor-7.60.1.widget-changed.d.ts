/**
 * A widget.
 * @beta
 */
export declare class Widget {
    size: number;
    render(): string;
    /** @beta */
    experimental(): void;
}
/** @public */
export interface Options {
    mode: Mode;
}
/** @public */
export declare enum Mode {
    Fast = 0,
    Slow = 1
}
/** @public */
export declare function create(o: Options, n: number): Widget;
/** @public */
export type Id = string;
/** @public */
export declare const VERSION = "1";
/** @internal */
export declare function _hidden(): void;
/** @public */
export declare function added(): void;

/**
 * A widget.
 * @public
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
export declare function create(o: Options): Widget;
/** @public */
export type Id = string;
/** @public */
export declare const VERSION = "1";
/** @alpha */
export declare function trial(): void;
/** @internal */
export declare function _hidden(): void;

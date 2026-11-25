/** @public */
export declare abstract class Base<T = string> {
    protected constructor(v: T);
    get value(): T;
    set value(v: T);
    static readonly KIND: "base";
    abstract run(): void;
    /** @beta */
    tune(n: number): this;
}
/** @public */
export declare class Child extends Base<number> implements Runnable {
    run(): void;
}
/** @public */
export interface Runnable {
    run(): void;
    readonly name?: string;
    [key: string]: unknown;
}
/** @public */
export type Shape = {
    kind: "a";
    size: number;
} | {
    kind: "b";
};
/** @public */
export declare function parse(input: string): number;
/** @public */
export declare function parse(input: number): string;
/** @public */
export declare namespace util {
    function helper(x: number): number;
    const LIMIT = 3;
}
/** @alpha */
export declare const enum Flags {
    A = 1,
    B = 2
}
export { Other } from './other';

//! `catalog!`, the macro that turns the catalog table (`catalog.rs`) into `CATALOG` and `codes`.

/// Generates `pub mod codes` (a constant for every core code) and
/// `pub const CATALOG: &[CodeEntry]` from a table of entries
/// `IDENT [= "ID"]: Level owner [machine], "title", "explanation";`. `owner` is `core`
/// or an extension's short name (`product` for `@specforge/product`); `machine` marks a code this machine
/// decides (its approvals, its network or its journal, ADR 0337 D7), never the project. The
/// one table is in `catalog.rs`; the macro is exported only so that its
/// compile-time checks can be shown in a doctest (see [`Code`](crate::Code)).
#[doc(hidden)]
#[macro_export]
macro_rules! catalog {
    (@id $ident:ident []) => {
        stringify!($ident)
    };
    (@id $ident:ident [$id:literal]) => {
        $id
    };
    (@owner core) => {
        "core"
    };
    (@owner $ext:ident) => {
        concat!("@specforge/", stringify!($ext))
    };
    (@machine) => {
        false
    };
    (@machine machine) => {
        true
    };
    // A core code whose pass sets its level.
    (@const $ident:ident [$($id:literal)?] SetByPass core [] $title:literal) => {
        #[doc = concat!("`", $crate::catalog!(@id $ident [$($id)?]), "`: ", $title, " (level set by its pass).")]
        pub const $ident: $crate::GradedCode =
            $crate::GradedCode::catalogued($crate::catalog!(@id $ident [$($id)?]));
    };
    // A core code at a fixed level.
    (@const $ident:ident [$($id:literal)?] $level:ident core [$($machine:ident)?] $title:literal) => {
        #[doc = concat!("`", $crate::catalog!(@id $ident [$($id)?]), "`: ", $title, " (", stringify!($level), ").")]
        pub const $ident: $crate::Code = $crate::Code::catalogued(
            $crate::catalog!(@id $ident [$($id)?]),
            $crate::Level::$level,
            $crate::catalog!(@machine $($machine)?),
        );
    };
    // An extension's code: no host constant, but its prefix is still checked.
    (@const $ident:ident [$($id:literal)?] $level:ident $ext:ident [$($machine:ident)?] $title:literal) => {
        const _: () = assert!(
            $crate::prefix_states($crate::catalog!(@id $ident [$($id)?]), $crate::Level::$level),
            "an E/W/I/A prefix states the code's level"
        );
    };
    ($( $ident:ident $(= $id:literal)? : $level:ident $owner:ident $($machine:ident)?, $title:literal, $explanation:literal; )*) => {
        /// A typed constant for every core code in [`CATALOG`]: `codes::W112`
        /// is a [`Code`](crate::Code) at the code's level, `codes::R_RES_005`
        /// names `R-RES-005` (`-` becomes `_`), and `codes::A010` is a
        /// [`GradedCode`](crate::GradedCode). Extension-owned codes have none:
        /// extensions report codes as text, checked where they enter the host.
        pub mod codes {
            $( $crate::catalog!(@const $ident [$($id)?] $level $owner [$($machine)?] $title); )*
        }

        /// Every catalogued code, sorted by code.
        pub const CATALOG: &[$crate::CodeEntry] = &[
            $( $crate::CodeEntry {
                code: $crate::catalog!(@id $ident [$($id)?]),
                title: $title,
                owner: $crate::catalog!(@owner $owner),
                level: $crate::Level::$level,
                machine: $crate::catalog!(@machine $($machine)?),
                explanation: $explanation,
            }, )*
        ];
    };
}

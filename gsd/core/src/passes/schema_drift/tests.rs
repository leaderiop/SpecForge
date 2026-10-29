use super::*;

#[test]
fn schema_patterns_are_gsds() {
    // Each of the ten patterns matches, and the neighbours that look alike do not.
    for (file, orm) in [
        ("src/collections/Posts.ts", "payload"),
        ("src/globals/Header.ts", "payload"),
        ("prisma/schema.prisma", "prisma"),
        ("prisma/schema/user.prisma", "prisma"),
        ("drizzle/schema.ts", "drizzle"),
        ("src/db/schema.ts", "drizzle"),
        ("drizzle/0001_init.ts", "drizzle"),
        ("supabase/migrations/001_init.sql", "supabase"),
        ("src/entities/User.ts", "typeorm"),
        ("src/migrations/1700-init.ts", "typeorm"),
        ("src/collections/deep/Posts.ts", "payload"),
        ("src\\entities\\User.ts", "typeorm"),
    ] {
        assert_eq!(orm_of(file), Some(orm), "{file}");
    }
    for file in [
        "src/collections/Posts.js",
        "xsrc/collections/Posts.ts",
        "prisma/schema.prisma.bak",
        "prisma/schema/user.ts",
        "supabase/migrations/001.ts",
        "src/db/schema.tsx",
        "src/entities.ts",
        "README.md",
    ] {
        assert_eq!(orm_of(file), None, "{file}");
    }
}

fn pushed(orm: &str, log: &str) -> bool {
    let orm = ORMS.iter().find(|o| o.name == orm).expect("an orm");
    has_evidence(orm, log)
}

#[test]
fn the_evidence_patterns_are_gsds() {
    assert!(pushed(
        "prisma",
        "ran npx prisma  db\tpush --accept-data-loss"
    ));
    assert!(pushed("prisma", "PRISMA MIGRATE DEPLOY"));
    assert!(pushed("prisma", "prisma migrate dev"));
    assert!(!pushed("prisma", "prisma migrate"));
    assert!(!pushed("prisma", "prismadb push"));
    assert!(pushed("payload", "CI=true npx payload migrate"));
    // PAYLOAD_MIGRATING is case-sensitive.
    assert!(pushed("payload", "PAYLOAD_MIGRATING=true"));
    assert!(!pushed("payload", "payload_migrating=true"));
    assert!(pushed("drizzle", "npx drizzle-kit push"));
    assert!(pushed("drizzle", "Drizzle-Kit migrate"));
    assert!(pushed("supabase", "supabase db push"));
    assert!(pushed("supabase", "supabase migration up"));
    assert!(pushed(
        "typeorm",
        "typeorm migration:run -d src/data-source.ts"
    ));
    assert!(pushed("typeorm", "typeorm schema:sync"));
    assert!(!pushed("typeorm", ""));
}

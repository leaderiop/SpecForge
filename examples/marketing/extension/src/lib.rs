//! @example/marketing: teaches SpecForge what a marketing plan is made of.
//!
//! Four kinds (`campaign`, `audience`, `message`, `brand_rule`), the links
//! between them, and four rules that catch real planning mistakes. One
//! command, `calendar`, lists the campaigns in date order.
//!
//! Build:    specforge extension build --dir examples/marketing/extension
//! Validate: specforge extension validate --dir examples/marketing/extension

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@example/marketing",
    version = "0.1.0",
    short = "marketing",
    description = "Campaigns, audiences, messages and brand rules, checked"
)]
struct Marketing;

/// Where a campaign runs.
const CHANNELS: Vocabulary = Vocabulary::new(&["email", "social", "search", "events", "partners"]);

impl Contributions for Marketing {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("brand_rule", |k| {
            k.description("A rule from the brand book that every message must obey")
                .testable(false);
            k.field("rule", |f| {
                f.field_type(FieldType::String)
                    .required()
                    .headline()
                    .description("The rule, as a writer would read it");
            });
            k.field("why", |f| {
                f.field_type(FieldType::String)
                    .description("Why the rule exists");
            });
        });

        c.kind("audience", |k| {
            k.description("A group of people a campaign speaks to")
                .testable(false);
            k.field("description", |f| {
                f.field_type(FieldType::String)
                    .required()
                    .headline()
                    .description("Who they are");
            });
            k.field("size", |f| {
                f.field_type(FieldType::Integer)
                    .description("How many people, roughly");
            });
        });

        c.kind("message", |k| {
            k.description("Something we say: a headline and its copy")
                .testable(false);
            k.field("headline", |f| {
                f.field_type(FieldType::String).required().headline();
            });
            k.field("body", |f| {
                f.field_type(FieldType::String);
            });
            k.field("obeys", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("MessageObeysBrandRule")
                    .target_kind("brand_rule")
                    .description("The brand rules this message was checked against");
            });
        });

        c.kind("campaign", |k| {
            k.description(
                "A push on one channel, to some audiences, with some messages, between two dates",
            )
            .testable(false);
            k.field("goal", |f| {
                f.field_type(FieldType::String)
                    .required()
                    .headline()
                    .description("The measurable outcome");
            });
            k.field("channel", |f| {
                f.vocabulary(&CHANNELS).required();
            });
            k.field("starts", |f| {
                f.field_type(FieldType::Date).required();
            });
            k.field("ends", |f| {
                f.field_type(FieldType::Date).required();
            });
            k.field("budget", |f| {
                f.field_type(FieldType::Integer).description("In euros");
            });
            k.field("audiences", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("CampaignTargetsAudience")
                    .target_kind("audience");
            });
            k.field("messages", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("CampaignUsesMessage")
                    .target_kind("message");
            });
        });

        // The links the reference fields above draw.
        for (label, source, target, description) in [
            (
                "CampaignTargetsAudience",
                "campaign",
                "audience",
                "who a campaign speaks to",
            ),
            (
                "CampaignUsesMessage",
                "campaign",
                "message",
                "what a campaign says",
            ),
            (
                "MessageObeysBrandRule",
                "message",
                "brand_rule",
                "the brand rules a message was checked against",
            ),
        ] {
            c.edge(label, |e| {
                e.source_kind(source)
                    .target_kind(target)
                    .description(description);
            });
        }

        // A campaign nobody receives.
        c.rule("E901", |r| {
            r.check(CheckKind::NoOutgoingEdges)
                .target_kind("campaign")
                .edge_type("CampaignTargetsAudience")
                .severity(ValidationSeverity::Error)
                .message_template("campaign '{id}' targets no audience")
                .description("A campaign that targets no audience: money spent on nobody. Give it `audiences`.");
        });

        // Copy that skipped the brand check.
        c.rule("E902", |r| {
            r.check(CheckKind::NoOutgoingEdges)
                .target_kind("message")
                .edge_type("MessageObeysBrandRule")
                .severity(ValidationSeverity::Error)
                .message_template("message '{id}' cites no brand rule")
                .description("A message that cites no brand rule: copy that skipped the brand check. Name the rules it `obeys`.");
        });

        // A calendar typo: the campaign ends before it starts.
        c.rule("E903", |r| {
            r.check(CheckKind::Custom)
                .target_kind("campaign")
                .severity(ValidationSeverity::Error)
                .message_template("campaign '{id}' ends ({value}) before it starts")
                .description("A campaign that ends before it starts: a calendar typo. Set `ends` on or after `starts`.")
                .validate(ends_before_it_starts);
        });

        // A rule in the brand book that no message follows.
        c.rule("W901", |r| {
            r.check(CheckKind::NoIncomingEdges)
                .target_kind("brand_rule")
                .edge_type("MessageObeysBrandRule")
                .severity(ValidationSeverity::Warning)
                .message_template("brand rule '{id}' is obeyed by no message")
                .description("A brand rule no message obeys: a rule nobody checks against. Cite it from a message, or remove it.");
        });

        c.command("calendar", |cmd| {
            cmd.title("Campaign calendar")
                .description("Every campaign in start-date order, with its channel and dates")
                .handler(|call| {
                    let mut rows: Vec<serde_json::Value> = call
                        .graph()
                        .nodes_of_kind("campaign")
                        .map(|n| {
                            serde_json::json!({
                                "id": n.id,
                                "channel": n.text("channel"),
                                "starts": n.text("starts"),
                                "ends": n.text("ends"),
                            })
                        })
                        .collect();
                    rows.sort_by(|a, b| a["starts"].as_str().cmp(&b["starts"].as_str()));
                    call.render(&serde_json::json!({ "campaigns": rows }), |out| {
                        for row in &rows {
                            let text = |k: &str| row[k].as_str().unwrap_or("?").to_string();
                            out.push_str(&format!(
                                "{} to {}  {:<8} {}\n",
                                text("starts"),
                                text("ends"),
                                text("channel"),
                                text("id")
                            ));
                        }
                    })
                });
        });
    }
}

/// E903's check. Dates are `YYYY-MM-DD`, so comparing the text compares the days.
fn ends_before_it_starts(context: &ValidatorContext) -> ValidatorVerdict {
    let field = |key: &str| {
        context
            .entity
            .fields
            .iter()
            .find(|f| f.key == key)
            .and_then(|f| f.value.as_str())
    };
    match (field("starts"), field("ends")) {
        (Some(starts), Some(ends)) if ends < starts => ValidatorVerdict::Fail {
            field: Some("ends".into()),
            value: Some(ends.into()),
        },
        _ => ValidatorVerdict::Pass,
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

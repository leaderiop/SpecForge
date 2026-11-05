// @specforge/checks: the registries family

use "extensions/checks/features"

behavior checks_registries "Registries Family" {
  features [ck_value_checks]
  category command
  ensures {
    registered   "A package, DOI or ORCID iD a value names is registered at its own resolver, else E105"
    resolvable   "A package's requirement admits a release that is not yanked, else E105 naming the newest release"
    recorded     "The release a requirement resolves to has no OSV advisory and a licence allow admits, else W179"
    outages_warn "A registry that is down, limits the rate, wants credentials or does not answer is W181, W182 or W172"
  }
  contract """
    Each registries checker (package, advisory, license_of, doi, orcid)
    MUST ask only the identifier's own public resolver, through the
    `registries` scope, whose hosts are fixed: index.crates.io,
    crates.io, registry.npmjs.org, pypi.org, api.osv.dev, doi.org and
    pub.orcid.org, at most 16 MiB of an answer. A package is written
    `crates:name@req`, `npm:[@scope/]name@range` or
    `pypi:Name[extras]specifiers`, and each ecosystem's own rules admit
    its versions, prereleases included. `package` MUST be E105 when the
    registry has no such package, or no release its requirement admits
    once yanked releases are left out, naming the newest release that is
    not yanked. `advisory` MUST then POST an OSV query for the release
    it resolved to and report each advisory not withdrawn as W179 with
    its aliases and the first fixed version; `license_of` MUST read the
    release's record and report W179 when its SPDX licence does not hold
    with the licences `allow` names, or when none is stated. `doi` is
    E105 for a DOI doi.org does not register and `orcid` for an iD ORCID
    has no record of. A malformed coordinate, DOI or ORCID iD is E103 and
    is not asked. An outage (5xx, 429) is W181, 401 or 403 W182, a
    redirect not followed W173, and no answer or one that does not read
    W172: the machine's, never E105. An
    answer cut at the read leaves the value unchecked (I206), never a
    pass: a large package's index says nothing of the project. Every answer is read into typed structures
    that skip what they do not read. The `families.registries` switch
    gates E105 and W179.
  """
  verify unit "a package asks its ecosystem's index, and a coordinate no registry names is E103"
  verify unit "a requirement only yanked releases meet is E105 on the requirement, naming the newest release"
  verify unit "an advisory check queries OSV for the release the requirement resolves to, and reports each advisory as W179"
  verify unit "a licence the release record states outside allow, or none, is W179"
  verify unit "a doi or an orcid is asked of its resolver, a malformed one is E103 and an unknown one E105"
  verify unit "an answer cut at the read leaves the value unchecked, never W172 nor a pass"
  verify integration "a requirement only yanked versions meet is E105 naming the newest"
  verify integration "a registry 503 is W181, never E105"
  verify integration "an OSV hit is W179 with its aliases, and a withdrawn one is not"
  verify integration "a licence outside allow is W179"
  verify integration "an unknown DOI is E105 and a malformed one is E103"
  verify integration "the registries scope asks only for its seven hosts"
  verify integration "an answer whose body the journal did not keep is a miss, never a pass"
}

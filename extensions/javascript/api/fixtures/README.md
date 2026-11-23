# @javascript/api fixtures

| Fixture | Source |
|---------|--------|
| `items.json` | hand-written served `item` facts (ids only) of `@demo/widget`'s `src/index.ts`: every member of the recorded model an item can be, except `Id`, which only another package (`@demo/other`) holds, so its `api_item` cites nothing |

The tests read the API reader's recorded doc model, `../../code/api/fixtures/api-extractor-7.60.1.widget.api.json`
(api-extractor 7.60.1, `@demo/widget`; see that directory's README).

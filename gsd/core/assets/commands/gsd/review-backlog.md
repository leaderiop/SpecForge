<!-- Ported from gsd-core v1.18.0 commands/gsd/review-backlog.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<objective>
Review all 999.x backlog items and optionally promote them into the active
milestone sequence or remove stale entries.
</objective>

<process>

1. **List backlog items:**
   ```bash
   _BACKLOG=$(specforge export --kinds phase --format graph | jq -r '.nodes[]|select(.fields.backlog)|.id')   # roadmap-wide: the backlog of every milestone
   if [ -n "$_BACKLOG" ]; then echo "$_BACKLOG"; else echo "No backlog items found"; fi
   ```

2. **Read the backlog phases** and extract all 999.x phase entries:
   ```bash
   for _ID in $_BACKLOG; do specforge query "$_ID" --depth 1 --format context; done
   ```
   Show each backlog item with its description, any accumulated context (CONTEXT.md, RESEARCH.md), and creation date.

3. **Present the list to the user** via AskUserQuestion:
   - For each backlog item, show: phase number, description, accumulated artifacts
   - Options per item: **Promote** (move to active), **Keep** (leave in backlog), **Remove** (delete)

4. **For items to PROMOTE:**
   - Find the next sequential phase number in the active milestone
   - Promote it with `specforge gsd phase --promote`: the phase takes the new number, and its context, research and plans are renamed with it. `<id>` is the item's full id from the list (`v1.0/999.1`); an item of another milestone moves under `$MILESTONE`:
     ```bash
     NEW_NUM=$(specforge gsd phase --promote "<id>" --milestone "$MILESTONE" --format json | jq -r '.output.to | split("/") | last')
     ```
   - The accumulated artifacts move with it (the command renames them)
   - The phase leaves the backlog by taking its new number
   - The backlog flag is computed from the number: there is no marker to remove
   - The command sets `depends_on` to the previous phase

5. **For items to REMOVE:**
   - Delete the phase with `specforge gsd phase --remove "999.x" --milestone "<its milestone>"`: it removes the phase and its artifacts and renumbers nothing
   - Nothing else to update: the backlog is the `999.x` phases

6. **Commit changes:**
   ```bash
   git add spec && git commit -m "docs: review backlog — promoted N, removed M" && specforge collect --runner gsd-git --yes
   ```

7. **Report summary:**
   ```
   ## 📋 Backlog Review Complete

   Promoted: {list of promoted items with new phase numbers}
   Kept: {list of items remaining in backlog}
   Removed: {list of deleted items}
   ```

</process>

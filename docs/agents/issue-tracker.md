# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for every operation. `gh` infers the repository from the clone, and `git remote -v` shows it is `OliverBrotchie/block-list`.

## Conventions

Create an issue with `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.

Read one issue with `gh issue view <number> --json number,title,body,labels,comments`.

List issues with:

```shell
gh issue list --state open --json number,title,body,labels,comments \
  --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'
```

Add `--label` and `--state` to narrow the list.

Make a new issue a child of another with `gh issue create --parent <parent> ...`. Link an existing issue with `gh issue edit <parent> --add-sub-issue <child>`. Both need `gh` 2.94 or newer, and this machine runs 2.103.3.

Comment with `gh issue comment <number> --body "..."`.

Apply and remove labels with `gh issue edit <number> --add-label "..."` and `gh issue edit <number> --remove-label "..."`.

Close with `gh issue close <number> --comment "..."`.

## Pull requests as a triage surface

PRs as a request surface: no.

Set the flag to yes if this repo treats external PRs as feature requests. `/triage` reads the flag. When it is yes, PRs run through the same labels and states as issues: `gh pr view <number> --comments` reads one, `gh pr diff <number>` shows the diff, and `gh pr comment`, `gh pr edit --add-label`, `gh pr close` act on it.

GitHub shares one number space across issues and pull requests, so a bare `#42` may be either. Resolve it with `gh pr view 42`, then fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Read it as in Conventions above.

## Wayfinding operations

`/wayfinder` uses these. The map is a single issue with child issues as tickets.

Create the map as one issue labelled `wayfinder:map`. Its body holds the Notes, Decisions so far, and fog sections.

Create a child ticket as an issue linked to the map as a GitHub sub-issue, using `--parent`. Label it `wayfinder:research`, `wayfinder:prototype`, `wayfinder:grilling`, or `wayfinder:task`. Assign the driver once the ticket is claimed. Where sub-issues are unavailable, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body.

Record blocking with GitHub issue dependencies, which the UI renders as the frontier. Add an edge with:

```shell
gh api --method POST repos/OliverBrotchie/block-list/issues/<child>/dependencies/blocked_by \
  -F issue_id=<blocker-db-id>
```

`<blocker-db-id>` is the blocker's numeric database id, from `gh api repos/OliverBrotchie/block-list/issues/<n> --jq .id`. It is not the `#number` and not the node id. A ticket is unblocked when every blocker is closed; GitHub reports open blockers in `issue_dependencies_summary.blocked_by`. Where dependencies are unavailable, put `Blocked by: #<n>` at the top of the child body.

Find the frontier among the map's open children. Drop any child with an open blocker or an assignee. The first one in map order wins.

Claim a ticket with `gh issue edit <n> --add-assignee @me` as the session's first write.

Resolve a ticket by commenting the answer with `gh issue comment <n> --body "..."`, closing it, then appending a gist and link to the map's Decisions so far.

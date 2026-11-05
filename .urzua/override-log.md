# Override log

Each row is an editor-hook override that let a blocked record edit through
(written by scripts/hooks/record-edit-hook.sh). A person sets Reviewed-by once they
have looked at it; `override.unreviewed` reports every row where it is blank.

| Date | Name | Record | Field | Rule | Reason | Reviewed-by |
|---|---|---|---|---|---|---|

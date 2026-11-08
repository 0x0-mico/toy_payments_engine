# Toy Payments Engine

## Decisions

`total` field could be derived but it could leave us in the position where total overflows if a user has a disputed Tx and later deposits more funds. For this reason plus the simplicity of outputing the same struct i chose to track total per client.

## Basics

## Completeness

## Correctness

## Safety & Robustness

## Efficiency

## Maintainability

## Testing



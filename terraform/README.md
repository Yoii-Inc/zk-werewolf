# Terraform — retired

This directory held zk-werewolf's ECS-on-Fargate stack in the `yoii-crypto-dev`
account (719037119908), applied through the HCP Terraform workspace `zk-werewolf-dev`.
The service moved to the shared dev EKS cluster (DEV-1878, DEV-1569) and that account
was closed; the workspace and every resource it described are gone.

zk-werewolf now runs from [`Yoii-Inc/yoii-gitops`](https://github.com/Yoii-Inc/yoii-gitops):
`kube/charts/zk-werewolf` (chart, values and its SOPS secrets, rotated in DEV-1892) and
`kube/argocd/dev/zk-werewolf.yaml`. It is scaled to zero as of 2026-09-10.

The two workflows that drove this directory — `deploy.yml` (Terraform apply, ECR push
and ECS update, all in the closed account) and `pr-check.yml` (Terraform plan) — were
removed with it. Both had failed on every run since the account closed.

To change infrastructure, open a pull request against yoii-gitops. Nothing added to
this directory would apply anywhere — this file is left in its place so that anyone
who navigates here finds where it went (DEV-2024).

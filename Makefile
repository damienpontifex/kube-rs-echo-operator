.SHELLFLAGS := -o errexit -o nounset -o pipefail 
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables 
MAKEFLAGS += --no-builtin-rules
.PHONY: all help dev ensure-cluster-is-running clean

## help: Display available commands and their descriptions
help:
	@echo "Usage:"
	@sed -n 's/^##//p' $(MAKEFILE_LIST) \
		| sort \
		| awk -v bold="$$(tput bold)" -v normal="$$(tput sgr0)" '{ $$1 = bold $$1 normal; print }' \
		| column -t -s ':'

ensure-cluster-is-running:
	@k3d cluster list --output json | jq --exit-status 'any(.name == "k3s-default")' > /dev/null \
		|| k3d cluster create

## dev: Run the operator in development mode with automatic rebuilds on file changes
dev: ensure-cluster-is-running
	watchexec --exts rs,toml --restart -- 'cargo build && kubectl apply --filename operator/echo-crd.yaml && RUST_LOG=info cargo run --package operator'

## clean: Delete the k3d cluster and cleanup any resources
clean:
	@k3d cluster delete || true

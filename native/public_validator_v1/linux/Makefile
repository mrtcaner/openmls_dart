# Run from the fork checkout root. This starts only a one-shot shell/validator,
# never the image's service entry point. Network is disabled.
.PHONY: verify
ARCH ?= amd64
ARTIFACT_ROOT ?= .buildlog/external-resync-r0-20261016/n3a/linux
SIZE_FIXTURES ?= .buildlog/external-resync-r0-20261016/n3a
verify:
	@test -n "$(RUNTIME_IMAGE)" || (echo 'Pass the exact local runtime image ID or pinned image digest' && exit 1)
	@test "$(ARCH)" = amd64 -o "$(ARCH)" = arm64
	docker run --rm --platform linux/$(ARCH) --network none --read-only --cap-drop ALL --security-opt no-new-privileges --memory 128m --cpus 1 --pids-limit 32 --user 10001:10001 --tmpfs /tmp:rw,noexec,nosuid,size=16m --mount "type=bind,src=$(CURDIR)/$(ARTIFACT_ROOT)/$(ARCH),dst=/artifact,readonly" --mount "type=bind,src=$(CURDIR)/native/public_validator_v1/fixtures,dst=/baseline,readonly" --mount "type=bind,src=$(CURDIR)/$(SIZE_FIXTURES),dst=/sizes,readonly" --entrypoint /bin/sh "$(RUNTIME_IMAGE)" -c 'set -eu; uname -m; cat /etc/alpine-release; cd /artifact; sha256sum -c artifact.sha256; for request in /baseline/*.request.bin /sizes/*.request.bin; do env -i /artifact/openmls_public_validator < "$$request" > /tmp/result 2> /tmp/stderr; cmp /tmp/result "$${request%.request.bin}.response.bin"; test ! -s /tmp/stderr; echo "PASS $$request (exact response, exit0, empty stderr)"; done'

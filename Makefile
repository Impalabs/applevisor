CODESIGN=codesign
CARGO=cargo
CARGO_NIGHTLY=$(CARGO) +nightly

ENTITLEMENTS=entitlements.xml
TARGET=applevisor
TARGET_DEBUG=target/debug/$(TARGET)
TARGET_RELEASE=target/release/$(TARGET)

.PHONY: build-tests tests build-tests-nightly tests-nightly

build-tests:
	$(CARGO) test --no-run
	$(CODESIGN) --sign - --entitlements "$(ENTITLEMENTS)" --deep --force \
		$(shell $(CARGO) test --no-run --message-format=json | \
			jq -r "select(.profile.test == true) | .filenames[]")

tests: build-tests
	$(CARGO) test --tests -- --nocapture --test-threads=1

build-tests-nightly:
	$(CARGO_NIGHTLY) test --features=simd_nightly --no-run
	$(CODESIGN) --sign - --entitlements "$(ENTITLEMENTS)" --deep --force \
		$(shell $(CARGO_NIGHTLY) test --features=simd_nightly --no-run --message-format=json | \
			jq -r "select(.profile.test == true) | .filenames[]")

tests-nightly: build-tests-nightly
	$(CARGO_NIGHTLY) test --tests --features=simd_nightly -- --nocapture --test-threads=1

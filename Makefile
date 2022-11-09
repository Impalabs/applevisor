CODESIGN=codesign
CARGO=cargo

TARGET=applevisor
TARGET_DEBUG=target/debug/$(TARGET)
TARGET_RELEASE=target/release/$(TARGET)

KEYCHAIN=$(CERT_KEYCHAIN)

build-debug:
	$(CARGO) fmt
	$(CARGO) build
	$(CODESIGN) --entitlements entitlements.xml -f -s "$(KEYCHAIN)" "$(TARGET_DEBUG)"

build-release:
	$(CARGO) fmt
	$(CARGO) build --release
	$(CODESIGN) --entitlements entitlements.xml -f -s "$(KEYCHAIN)" "$(TARGET_RELEASE)"

build-test:
	$(CARGO) test --no-run
	$(CODESIGN) --entitlements entitlements.xml -f -s "$(KEYCHAIN)" \
		$(shell $(CARGO) test --no-run --message-format=json | \
			jq -r "select(.profile.test == true) | .filenames[]")

tests: build-test
	$(CARGO) test --tests -- --nocapture --test-threads=1

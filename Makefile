CODESIGN=codesign
CARGO=cargo
CARGO_NIGHTLY=$(CARGO) +nightly
LLVM_PROFDATA=llvm-profdata
LLVM_COV=llvm-cov

ENTITLEMENTS=entitlements.xml

.PHONY: tests-stable-macos-11-0 \
		tests-stable-macos-12-1 \
		tests-stable-macos-13-0 \
		tests-stable-macos-15-0 \
		tests-stable-macos-15-2 \
		tests-stable-macos-26-0 \
		tests-stable-all \
		tests-nightly-macos-11-0 \
		tests-nightly-macos-12-1 \
		tests-nightly-macos-13-0 \
		tests-nightly-macos-15-0 \
		tests-nightly-macos-15-2 \
		tests-nightly-macos-26-0 \
		tests-nightly-all \
		tests-all \
		coverage-macos-11-0 \
		coverage-macos-12-1 \
		coverage-macos-13-0 \
		coverage-macos-15-0 \
		coverage-macos-15-2 \
		coverage-macos-26-0

define stable_macos_version_test
	$(CARGO) \
		test $(3) \
		--no-default-features \
		--features=macos-$(1)-$(2) \
		--no-run
	$(CODESIGN) \
		--sign - \
		--entitlements "$(ENTITLEMENTS)" \
		--deep \
		--force \
			$(shell $(CARGO) \
				test $(3) \
				--no-default-features \
				--features=macos-$(1)-$(2) \
				--no-run --message-format=json \
				| jq -r "select(.profile.test == true) | .filenames[]")
	$(CARGO) \
		test $(3) \
		--no-default-features \
		--features=macos-$(1)-$(2) \
		-- --nocapture
endef

tests-stable-macos-11-0:
	@$(call stable_macos_version_test,11,0)

tests-stable-macos-12-1:
	@$(call stable_macos_version_test,12,1)

tests-stable-macos-13-0:
	@$(call stable_macos_version_test,13,0)

tests-stable-macos-15-0:
	@$(call stable_macos_version_test,15,0)

tests-stable-macos-15-2:
	@$(call stable_macos_version_test,15,2)

tests-stable-macos-26-0:
	@$(call stable_macos_version_test,26,0)

tests-stable-all: \
	tests-stable-macos-11-0 \
	tests-stable-macos-12-1 \
	tests-stable-macos-13-0 \
	tests-stable-macos-15-0 \
	tests-stable-macos-15-2 \
	tests-stable-macos-26-0


define nightly_macos_version_test
	$(CARGO_NIGHTLY) \
		test $(3) \
		--no-default-features \
		--features=macos-$(1)-$(2),simd-nightly \
		--no-run
	$(CODESIGN) \
		--sign - \
		--entitlements "$(ENTITLEMENTS)" \
		--deep \
		--force \
			$(shell $(CARGO_NIGHTLY) \
				test \
				--no-default-features \
				--features=macos-$(1)-$(2),simd-nightly \
				--no-run \
				--message-format=json \
				| jq -r "select(.profile.test == true) | .filenames[]")
	$(CARGO_NIGHTLY) \
		test $(3) \
		--no-default-features \
		--features=macos-$(1)-$(2),simd-nightly \
		-- --nocapture
endef

tests-nightly-macos-11-0:
	@$(call nightly_macos_version_test,11,0)

tests-nightly-macos-12-1:
	@$(call nightly_macos_version_test,12,1)

tests-nightly-macos-13-0:
	@$(call nightly_macos_version_test,13,0)

tests-nightly-macos-15-0:
	@$(call nightly_macos_version_test,15,0)

tests-nightly-macos-15-2:
	@$(call nightly_macos_version_test,15,2)

tests-nightly-macos-26-0:
	@$(call nightly_macos_version_test,26,0)

tests-nightly-all: \
	tests-nightly-macos-11-0 \
	tests-nightly-macos-12-1 \
	tests-nightly-macos-13-0 \
	tests-nightly-macos-15-0 \
	tests-nightly-macos-15-2 \
	tests-nightly-macos-26-0


tests-all: tests-stable-all tests-nightly-all


define macos_version_test_coverage
	LLVM_PROFILE_FILE="coverage/macos-$(1)-$(2)/%p.profraw" RUSTFLAGS="-C instrument-coverage" $(CARGO) \
		test $(3):: \
		--no-default-features \
		--features=macos-$(1)-$(2) \
		--no-run
	$(CODESIGN) \
		--sign - \
		--entitlements "$(ENTITLEMENTS)" \
		--deep \
		--force \
		$(shell LLVM_PROFILE_FILE="coverage/macos-$(1)-$(2)/%p.profraw" RUSTFLAGS="-C instrument-coverage" $(CARGO) \
			test $(3):: \
			--no-default-features \
			--features=macos-$(1)-$(2) \
			--no-run --message-format=json \
			| jq -r "select(.profile.test == true) | .filenames[]")
	rm -rf coverage/macos-$(1)-$(2)
	mkdir -p coverage/macos-$(1)-$(2)
	cd coverage/macos-$(1)-$(2); \
	LLVM_PROFILE_FILE="coverage/macos-$(1)-$(2)/%p.profraw" RUSTFLAGS="-C instrument-coverage" $(CARGO) \
		test $(3):: \
		--no-default-features \
		--features=macos-$(1)-$(2) \
		-- --nocapture; \
	$(LLVM_PROFDATA) merge \
		-sparse *.profraw \
		-o profdata; \
	$(LLVM_COV) show \
		-instr-profile=profdata \
		-object $(shell RUSTFLAGS="-C instrument-coverage" $(CARGO) \
			test $(3):: \
			--no-default-features \
			--features=macos-$(1)-$(2) \
			--no-run --message-format=json \
			| jq -r "select(.profile.test == true) | .filenames[]") \
		--format=html \
		-o output;
	open coverage/macos-$(1)-$(2)/output/index.html
endef

coverage-macos-11-0:
	@$(call macos_version_test_coverage,11,0)

coverage-macos-12-1:
	@$(call macos_version_test_coverage,12,1)

coverage-macos-13-0:
	@$(call macos_version_test_coverage,13,0)

coverage-macos-15-0:
	@$(call macos_version_test_coverage,15,0)

coverage-macos-15-2:
	@$(call macos_version_test_coverage,15,2)

coverage-macos-26-0:
	@$(call macos_version_test_coverage,26,0)

clean:
	make -C applevisor-sys clean
	$(CARGO) clean
	rm -rf coverage/

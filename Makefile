ifeq ($(USER),1)
PREFIX := $(HOME)/.local
BINDIR := $(HOME)/.local/bin
SHAREDIR := $(HOME)/.config
SYSTEMD_DIR := $(HOME)/.config/systemd/user
else
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
SHAREDIR ?= $(PREFIX)/share
SYSTEMD_DIR ?= $(PREFIX)/lib/systemd/user
endif

.PHONY: install install_systemd clean

target/release/track:
	cargo build --release

install: target/release/track
	install -d $(BINDIR)
	install -m 0755 target/release/track $(BINDIR)/track

install_systemd: install
	install -d $(SYSTEMD_DIR)
	install -m 0644 track-notify.service $(SYSTEMD_DIR)/track-notify.service
	systemctl --user daemon-reload || true

clean:
	cargo clean

#### Completions

.PHONY: install_completions_bash
install_completions_bash: target/release/track
	install -d $(SHAREDIR)/bash-completion/completions
	target/release/track --completion bash > $(SHAREDIR)/bash-completion/completions/track

.PHONY: install_completions_elvish
install_completions_elvish: target/release/track
	install -d $(SHAREDIR)/elvish/completions
	target/release/track --completion elvish > $(SHAREDIR)/elvish/completions/track

.PHONY: install_completions_fish
install_completions_fish: target/release/track
	install -d $(SHAREDIR)/fish/completions
	target/release/track --completion fish > $(SHAREDIR)/fish/completions/track.fish

.PHONY: install_completions_powershell
install_completions_powershell: target/release/track
	install -d $(SHAREDIR)/powershell/completions
	target/release/track --completion powershell > $(SHAREDIR)/powershell/completions/track.ps1

.PHONY: install_completions_zsh
install_completions_zsh: target/release/track
	install -d $(SHAREDIR)/zsh/site-functions
	target/release/track --completion zsh > $(SHAREDIR)/zsh/site-functions/_track

# ===========================================================================
# Install a Makefile-driven service as a systemd unit
#
# Drop this file and systemd/service.service.in into any project and include it
# from the Makefile. The generated unit only ever calls make goals in this
# folder, so what the service *is* stays defined in the Makefile.
#
# Required goals (the unit calls them):
#   $(SERVICE_START_GOAL)   start the service and return
#   $(SERVICE_STOP_GOAL)    stop it
#
# Optional goals:
#   $(SERVICE_RELOAD_GOAL)  used for `systemctl reload` (defaults to the start goal)
#   $(SERVICE_STATUS_GOAL)  used by `make service-status` when it exists
#
# Usage:
#   make init-system        is systemd in charge of this machine?
#   make service-check      are the required goals present?
#   make service-show       print the unit that would be installed
#   make service-install    install, enable and start it (sudo)
#   make service-status     unit state plus the project's own status goal
#   make service-logs       the unit's own log (its start and stop runs)
#   make service-uninstall  stop, disable and remove the unit (sudo)
#
# Any variable below can be overridden by the including Makefile (assign it
# before the include, `?=` keeps the first definition) or on the command line:
#
#   make service-install SERVICE=tcms SERVICE_START_GOAL=docker-up
#
# After installing, drive the service through systemctl rather than calling the
# make goals by hand, so systemd's view of it stays correct.
# ===========================================================================

SERVICE                ?= $(notdir $(CURDIR))
SERVICE_DESCRIPTION    ?= $(SERVICE) (started by make in $(CURDIR))
SERVICE_START_GOAL     ?= start
SERVICE_STOP_GOAL      ?= stop
SERVICE_RELOAD_GOAL    ?= $(SERVICE_START_GOAL)
SERVICE_STATUS_GOAL    ?= status
# Units the service depends on, and units to enable at boot alongside it.
# Set to empty for a service that needs neither.
SERVICE_REQUIRES       ?= docker.service
SERVICE_AFTER          ?= docker.service network-online.target
SERVICE_ENABLE_UNITS   ?= docker.service
SERVICE_TIMEOUT_SEC    ?= 1800
SERVICE_TEMPLATE       ?= systemd/service.service.in
SERVICE_FILE           := /etc/systemd/system/$(SERVICE).service
SERVICE_MAKE_BIN       ?= $(shell command -v $(firstword $(MAKE)) 2>/dev/null || command -v make)
SERVICE_GOALS_REQUIRED := $(sort $(SERVICE_START_GOAL) $(SERVICE_STOP_GOAL) $(SERVICE_RELOAD_GOAL))

# '|' as the sed separator: paths contain '/', and '#' would start a make comment.
SERVICE_RENDER = sed \
	-e "s|@DESCRIPTION@|$(SERVICE_DESCRIPTION)|g" \
	-e "s|@REQUIRES@|$(SERVICE_REQUIRES)|g" \
	-e "s|@AFTER@|$(SERVICE_AFTER)|g" \
	-e "s|@WORKDIR@|$(CURDIR)|g" \
	-e "s|@MAKE@|$(SERVICE_MAKE_BIN)|g" \
	-e "s|@START_GOAL@|$(SERVICE_START_GOAL)|g" \
	-e "s|@STOP_GOAL@|$(SERVICE_STOP_GOAL)|g" \
	-e "s|@RELOAD_GOAL@|$(SERVICE_RELOAD_GOAL)|g" \
	-e "s|@TIMEOUT@|$(SERVICE_TIMEOUT_SEC)|g" \
	$(SERVICE_TEMPLATE)

# ---------------------------------------------------------------------------
# Is systemd in charge of this machine?
# ---------------------------------------------------------------------------
init-system:
	@echo "PID 1 command:    $$(ps -p 1 -o comm= 2>/dev/null)"
	@echo "/sbin/init ->     $$(readlink -f /sbin/init 2>/dev/null || echo '(none)')"
	@if [ -d /run/systemd/system ]; then \
		echo "Init system:      systemd"; \
		echo "Version:          $$(systemctl --version 2>/dev/null | head -1)"; \
		echo "Verdict:          'make service-install' can be used here"; \
	else \
		echo "Init system:      NOT systemd (no /run/systemd/system)"; \
		echo "Verdict:          this installer does not apply on this machine"; \
	fi
.PHONY: init-system

# ---------------------------------------------------------------------------
# Do we have the goals the unit needs?
# ---------------------------------------------------------------------------
service-check:
	@missing=""; \
	for goal in $(SERVICE_GOALS_REQUIRED); do \
		$(MAKE) --no-print-directory -n "$$goal" >/dev/null 2>&1 || missing="$$missing $$goal"; \
	done; \
	if [ -n "$$missing" ]; then \
		echo "Goal(s) not found in $(CURDIR)/Makefile (or not dry-runnable):$$missing"; \
		echo "Define them, or point the installer at the goals this project has:"; \
		echo "  make service-install SERVICE_START_GOAL=<goal> SERVICE_STOP_GOAL=<goal>"; \
		exit 1; \
	fi; \
	echo "goals ok: start='$(SERVICE_START_GOAL)' stop='$(SERVICE_STOP_GOAL)' reload='$(SERVICE_RELOAD_GOAL)'"
.PHONY: service-check

service-show: ## Print the unit that service-install would write
	@test -f $(SERVICE_TEMPLATE) || { echo "Missing template: $(SERVICE_TEMPLATE)"; exit 1; }
	@echo "# would be installed as $(SERVICE_FILE)"
	@$(SERVICE_RENDER)
.PHONY: service-show

service-install: service-check ## Install, enable and start the unit (sudo)
	@test -n "$(SERVICE_MAKE_BIN)" || { echo "make not found in PATH"; exit 1; }
	@test -f $(SERVICE_TEMPLATE) || { echo "Missing template: $(SERVICE_TEMPLATE)"; exit 1; }
	$(SERVICE_RENDER) | sudo tee $(SERVICE_FILE) > /dev/null
	@for unit in $(SERVICE_ENABLE_UNITS); do echo "+ enabling $$unit"; sudo systemctl enable "$$unit"; done
	sudo systemctl daemon-reload
	sudo systemctl enable --now $(SERVICE).service
	@echo "Installed $(SERVICE_FILE). $(SERVICE) now starts at boot."
	@echo "Check with: make service-status"
.PHONY: service-install

service-uninstall: ## Stop, disable and remove the unit (sudo)
	-sudo systemctl disable --now $(SERVICE).service
	sudo rm -f $(SERVICE_FILE)
	sudo systemctl daemon-reload
	@echo "Removed $(SERVICE_FILE). The project's own data is untouched."
.PHONY: service-uninstall

service-status: ## Unit state, plus the project's status goal when it has one
	-systemctl status $(SERVICE).service --no-pager
	@if $(MAKE) --no-print-directory -n $(SERVICE_STATUS_GOAL) >/dev/null 2>&1; then \
		$(MAKE) --no-print-directory $(SERVICE_STATUS_GOAL); \
	else \
		echo "(no '$(SERVICE_STATUS_GOAL)' goal in this Makefile)"; \
	fi
.PHONY: service-status

service-logs: ## The unit's own log: its start and stop runs
	journalctl -u $(SERVICE).service --no-pager -n 100
.PHONY: service-logs

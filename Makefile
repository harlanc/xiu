COPY_CONF_FILES = sh ./update_project_conf.sh

not_spport:
	echo "input make <local|online|clean|check>"
# build local source codes
local:
	cd ./confs && $(COPY_CONF_FILES) "local"
	cargo build
# pull the online crates codes and build
online:
	cd ./confs && $(COPY_CONF_FILES) "online"
	cargo build
check:
	cargo clippy --fix --allow-dirty --allow-no-vcs
clean:
	cargo clean
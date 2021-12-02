#!/bin/sh
# Gnew test library, based on the Git test framework.

red=$(tput bold; tput setaf 1)
green=$(tput setaf 2)
reset=$(tput sgr0)

cd "$(mktemp -d)" || exit
gnew init

test_expect_success () {
	echo "test '$1': $2"
	if eval "$2"; then
		echo "${green}ok${reset} - $1"
	else
		echo "${red}FAIL${reset} - $1"
	fi
}

test_commit () {
	echo "$1" >"$2" &&
	gnew add "$2" &&
	gnew commit "$1"
}

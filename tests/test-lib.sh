#!/bin/sh
# Gnew test library, based on the Git test framework.

USER=G7

red=$(tput bold; tput setaf 1)
green=$(tput setaf 2)
reset=$(tput sgr0)

if test "$VERBOSE"
then
	exec 3>&1
else
	exec 3>/dev/null
fi

rm -rf testrun
mkdir testrun && cd testrun || exit
gnew init 2>&3 >&3

test_expect_success () {
	echo "test '$1': $2" >&3

	if eval "$2" 2>&3 >&3
	then
		echo "${green}ok${reset} - $1"
	else
		echo "${red}FAIL${reset} - $1"
	fi
}

test_commit () {
	echo "$1" >"$2" &&
	gnew add "$2" >/dev/null &&
	gnew commit "$1"
}

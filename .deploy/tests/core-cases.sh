# shellcheck shell=bash
#
# The cases for lib/core.sh: the shell options, logging, die, require, need_value, unknown_option
# and the error trap, through scripts that source lib.sh as the deploy scripts do. Needs LIB, the
# path of the lib.sh to test, and TEMP_DIR, a directory to write those scripts in, and the
# assertions. Sourced by the lib suite, never run. Byte for byte the same as its copy in the other
# repository that deploys Unipept, as core.sh is.
# shellcheck disable=SC2016 # the bodies are the scripts' own code, expanded when those run

# A script that sources lib.sh, as the deploy scripts do, and then runs the body. Its own process,
# since die signals the process that sourced lib.sh.
core_script() {
    local script="${TEMP_DIR}/$1"

    printf '#!/usr/bin/env bash\nsource %q\n%s\n' "$LIB" "$2" > "$script"
    chmod +x "$script"
    echo "$script"
}


section "core.sh: the shell options"

script=$(core_script options.sh 'echo "$-"; shopt -o pipefail | awk "{ print \$2 }"')
output=$("$script")
check_true "a script stops at a command that fails" grep -q e <<< "$output"
check_true "and at a variable that was never set" grep -q u <<< "$output"
check_true "and runs the error trap in functions and subshells" grep -q E <<< "$output"
check_true "and a pipeline fails where any part of it does" grep -qx on <<< "$output"

script=$(core_script unset.sh 'echo "${never_set}"
echo "went on"')
"$script" > "${TEMP_DIR}/output" 2>&1
check_true "a variable that was never set stops the script" test "$?" -ne 0
check_absent "before the next line" "went on" "${TEMP_DIR}/output"


section "core.sh: log"

script=$(core_script logs.sh 'log "a message"')
# In a time zone five hours from UTC, so a stamp in local time cannot pass for one in UTC. Read
# before and after, in case the minute turns in between.
before=$(date -u +'%F %H:%M')
TZ=Etc/GMT-5 "$script" > "${TEMP_DIR}/stdout" 2> "${TEMP_DIR}/stderr"
after=$(date -u +'%F %H:%M')
check "log writes nothing on standard output" "$(cat "${TEMP_DIR}/stdout")" ""
check_true "and the line, stamped with the date and time, on standard error" \
    grep -Eqx '[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}Z  a message' "${TEMP_DIR}/stderr"
stamp=$(cut -c 1-16 "${TEMP_DIR}/stderr")
check_true "in UTC" test "$stamp" = "$before" -o "$stamp" = "$after"


section "core.sh: die"

script=$(core_script dies.sh 'die "a reason"
echo "went on"')
"$script" > "${TEMP_DIR}/stdout" 2> "${TEMP_DIR}/stderr"
check "die stops the script with status 2" "$?" "2"
check "saying why, on standard error" "$(cat "${TEMP_DIR}/stderr")" "Error: a reason"
check "and nothing more" "$(cat "${TEMP_DIR}/stdout")" ""

script=$(core_script dies-inside.sh 'version=$(die "inside a substitution")
echo "went on with ${version}"')
output=$("$script" 2>&1)
check "a die inside a command substitution stops the script itself" "$?" "2"
check "and is reported once, by die alone" "$output" "Error: inside a substitution"

script=$(core_script dies-deeper.sh 'outer() { local inner; inner=$(die "two subshells down"); echo "$inner"; }
echo "$(outer)"
echo "went on"')
output=$("$script" 2>&1)
check "as does one two subshells down" "$?" "2"
check "reported once" "$output" "Error: two subshells down"


section "core.sh: require"

script=$(core_script requires.sh 'require bash no-such-tool-here other-missing-tool')
output=$("$script" 2>&1)
check "a command that is not there stops the script" "$?" "2"
check "and is named, the first one missing" "$output" "Error: no-such-tool-here is not installed."

script=$(core_script requires-package.sh 'require "no-such-tool-here:the tool package"')
output=$("$script" 2>&1)
check "with what to install, where that is named" "$output" \
    "Error: no-such-tool-here is not installed. It comes with the tool package."

script=$(core_script requires-present.sh 'require bash sh:dash
echo "went on"')
check "commands that are there let the script go on" "$("$script" 2>&1)" "went on"


section "core.sh: need_value"

script=$(core_script needs-value.sh 'need_value --flag "${1-}"; echo "went on"')
check "a flag with a value goes on" "$("$script" value 2>&1)" "went on"
output=$("$script" 2>&1)
check "a flag with nothing after it stops the script" "$?" "2"
check "and says so" "$output" "Error: --flag requires a value."
"$script" --other-flag > /dev/null 2>&1
check "as does a flag with another flag after it" "$?" "2"


section "core.sh: unknown_option"

script=$(core_script unknown-option.sh 'unknown_option --no-such-option
echo "went on"')
output=$("$script" 2>&1)
check "an option the script does not take stops it with status 2" "$?" "2"
check "and says where to look" "$output" "Error: unknown option '--no-such-option'. Run with --help for the options."


section "core.sh: the error trap"

script=$(core_script fails.sh 'false')
output=$("$script" 2>&1)
check "a command that fails where nothing expected it stops the script with status 2" "$?" "2"
check "and names the script, the command, and the line of the file it is on" "$output" \
    "Error: fails.sh stopped: 'false' failed with exit status 1 at line 3 of ${script}."

printf 'fails_in_part() {\n    false\n}\n' > "${TEMP_DIR}/part.sh"
script=$(core_script fails-in-part.sh "source '${TEMP_DIR}/part.sh'
fails_in_part")
output=$("$script" 2>&1)
check "a command that fails in a sourced file stops the script" "$?" "2"
check "and names that file, not the script, as where" "$output" \
    "Error: fails-in-part.sh stopped: 'false' failed with exit status 1 at line 2 of ${TEMP_DIR}/part.sh."

output=$(bash -c 'source "$1"; false' fails-in-bash-c "$LIB" 2>&1)
check "a command that fails in code given to bash -c stops it with status 2" "$?" "2"
check "and names it" "$output" "Error: fails-in-bash-c stopped: 'false' failed with exit status 1 at line 1 of fails-in-bash-c."

script=$(core_script fails-in-pipeline.sh 'false | cat')
"$script" > /dev/null 2>&1
check "as does a command that fails in the middle of a pipeline" "$?" "2"

script=$(core_script fails-inside.sh 'inner() { false; echo "went on"; }
result=$(inner)')
output=$("$script" 2>&1)
check "a command that fails inside a command substitution stops the script" "$?" "2"
check "and is reported once, at the line that started it" "$output" \
    "Error: fails-inside.sh stopped: 'result=\$(inner)' failed with exit status 1 at line 4 of ${script}."

script=$(core_script expected-inside.sh 'read -r answer < <(false) || true
x=$(false) || true
if ! false; then echo "went on"; fi')
output=$("$script" 2>&1)
check "a failure the script expects, inside a substitution or a condition, does not stop it" "$?" "0"
check "nor is it reported" "$output" "went on"

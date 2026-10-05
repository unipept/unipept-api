# shellcheck shell=bash
#
# The locks that keep the deploy scripts on one host from working on the same thing at once. Empty
# for now: the API lock, which deploy.sh takes for a deploy, a rollback, a start and a stop, and
# which unipept-database's switch.sh hands it, comes here with the deploy.sh status contract. The
# rollout's own lock moves here then too. unipept-database has a part of the same name, for its own
# locks. Sourced through .deploy/lib.sh.


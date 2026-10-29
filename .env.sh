[[ -f "Makefile" ]] && [[ 0 -eq $(grep -q "test:" Makefile) ]]

[[ -f ".env" ]] && set -a && source .env && set +a

export GIT_AUTHOR_EMAIL="alvarogarcia7@users.noreply.github.com"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"


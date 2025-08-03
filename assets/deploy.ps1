git push
if (-not $?)
{
    throw 'Native Failure'
}

ssh fenhl.net rustup update stable
if (-not $?)
{
    throw 'Native Failure'
}

ssh fenhl.net cargo install-update -fg molecule-db
if (-not $?)
{
    throw 'Native Failure'
}

ssh fenhl.net env -C /opt/git/github.com/fenhl/molecule-db/main git pull
if (-not $?)
{
    throw 'Native Failure'
}

ssh fenhl.net sudo systemctl restart molecule-db
if (-not $?)
{
    throw 'Native Failure'
}

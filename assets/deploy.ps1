git push
if (-not $?)
{
    throw 'Native Failure'
}

ssh mol.fenhl.net env -C git/github.com/fenhl/molecule-db/main git pull
if (-not $?)
{
    throw 'Native Failure'
}

dev dushanbe update
if (-not $?)
{
    throw 'Native Failure'
}

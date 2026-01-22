const canvas = document.getElementById('current');
const nextCanvas = document.getElementById('next');
const drawerWidth = 264;
const width = 1001;
const height = 652;
nextCanvas.width = canvas.width = width * window.devicePixelRatio;
nextCanvas.height = canvas.height = height * window.devicePixelRatio;
canvas.style.width = `${canvas.width / window.devicePixelRatio}px`;
canvas.style.height = `${canvas.height / window.devicePixelRatio}px`;

let mouseX = 0;
let mouseY = 0;
let prevState = null;
let state = {
    'selectedAtom': 'salt',
    'selectedBond': 'n',
};
let nextState = state;

const atomsByEncoding = [
    'salt',
    'air',
    'earth',
    'fire',
    'water',
    'quicksilver',
    'gold',
    'silver',
    'copper',
    'iron',
    'tin',
    'lead',
    'vitae',
    'mors',
    'quintessence',
];
const atomEncoding = {
    salt: 0n,
    air: 1n,
    earth: 2n,
    fire: 3n,
    water: 4n,
    quicksilver: 5n,
    gold: 6n,
    silver: 7n,
    copper: 8n,
    iron: 9n,
    tin: 10n,
    lead: 11n,
    vitae: 12n,
    mors: 13n,
    quintessence: 14n,
};
function bondEncoding(bond) {
    if (bond === 'n')
        return 1n;
    else if (bond === 'ryk')
        return 2n;
    else
        return 0n;
}

function visit(atom, bond) {
    for (let i = -4; i <= 4; ++i) {
        for (let j = -4; j <= 4; ++j) {
            const x = drawerWidth + (width - drawerWidth) / 2 + 82 * (i + 0.5 * j);
            const y = height / 2 - 71 * j;
            if (i + j < -4 || i + j > 4)
                continue;
            if (atom)
                atom(i, j, x, y);
            if (bond) {
                bond(i, j, i + 1, j, x + 41, y, 0);
                bond(i, j, i - 1, j + 1, x - 20.5, y - 35.5, 1/6);
                bond(i, j, i, j + 1, x + 20.5, y - 35.5, 2/6);
            }
        }
    }
}
function visitDrawer(atom, bond) {
    const spacing = 82;
    const x0 = 50;
    const x1 = x0 + spacing;
    const x2 = x1 + spacing;
    if (bond) {
        bond(x0, 55 + spacing * 5, 'n');
        bond(x1, 55 + spacing * 5, 'ryk');
        bond(x0, 55 + spacing * 5 + 71, 'r');
        bond(x1, 55 + spacing * 5 + 71, 'k');
        bond(x2, 55 + spacing * 5 + 71, 'y');
        bond(x0, 55 + spacing * 5 + 71 * 2, 'ky');
        bond(x1, 55 + spacing * 5 + 71 * 2, 'ry');
        bond(x2, 55 + spacing * 5 + 71 * 2, 'rk');
    }
    if (atom) {
        atom(x0, 55, 'salt');
        atom(x1, 55, 'air');
        atom(x2, 55, 'fire');
        atom(x0, 55 + spacing, 'quicksilver');
        atom(x1, 55 + spacing, 'water');
        atom(x2, 55 + spacing, 'earth');
        atom(x0, 55 + spacing * 2, 'lead');
        atom(x1, 55 + spacing * 2, 'tin');
        atom(x2, 55 + spacing * 2, 'iron');
        atom(x0, 55 + spacing * 3, 'copper');
        atom(x1, 55 + spacing * 3, 'silver');
        atom(x2, 55 + spacing * 3, 'gold');
        atom(x0, 55 + spacing * 4, 'vitae');
        atom(x1, 55 + spacing * 4, 'mors');
        atom(x2, 55 + spacing * 4, 'quintessence');
        atom(x2, 55 + spacing * 5, 'repeat');
    }
}

// from https://stackoverflow.com/a/7838871
function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x+r, y);
  ctx.arcTo(x+w, y,   x+w, y+h, r);
  ctx.arcTo(x+w, y+h, x,   y+h, r);
  ctx.arcTo(x,   y+h, x,   y,   r);
  ctx.arcTo(x,   y,   x+w, y,   r);
  ctx.closePath();
}

function draw(canvas, state) {
    const isLight = window.matchMedia('(prefers-color-scheme: light)').matches;
    const ctx = canvas.getContext('2d');
    ctx.save();
    ctx.fillStyle = isLight ? '#ccd' : '#223';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
    ctx.fillStyle = isLight ? '#c4c4d5' : '#2a2a3b';
    ctx.fillRect(0, 0, drawerWidth, height);
    visit(null, function (i0, j0, i1, j1, x, y, rotation) {
        const bond = state[`${i0},${j0}:${i1},${j1}`];
        if (!bond)
            return;
        ctx.save();
        ctx.translate(x + 4, y + 4);
        ctx.rotate(rotation * 2 * Math.PI);
        drawBond(ctx, bond.includes('r'), bond.includes('k'), bond.includes('y'), 82, 'shadow');
        ctx.restore();
    });
    visit(function (i, j, x, y) {
        const atom = state[`${i},${j}`];
        if (!atom) {
            ctx.fillStyle = isLight ? '#dde' : '#112';
            ctx.fillRect(x - 1, y - 1, 4, 4);
            return;
        }
        ctx.fillStyle = atomStyle[atom].shadowStyle;
        ctx.beginPath();
        ctx.ellipse(x + 4, y + 4, 29, 29, 0, 0, 2 * Math.PI);
        ctx.fill();
    });
    visit(null, function (i0, j0, i1, j1, x, y, rotation) {
        const bond = state[`${i0},${j0}:${i1},${j1}`];
        if (!bond)
            return;
        ctx.save();
        ctx.translate(x + 2, y + 2);
        ctx.rotate(rotation * 2 * Math.PI);
        drawBond(ctx, bond.includes('r'), bond.includes('k'), bond.includes('y'), 82);
        ctx.restore();
    });
    visit(function (i, j, x, y) {
        const atom = state[`${i},${j}`];
        if (!atom)
            return;
        ctx.fillStyle = atomStyle[atom].shadowStyle;
        ctx.beginPath();
        ctx.ellipse(x + 2, y + 2, 29, 29, 0, 0, 2 * Math.PI);
        ctx.fill();
    });
    visit(function (i, j, x, y) {
        const atom = state[`${i},${j}`];
        if (!atom)
            return;
        drawAtom(ctx, atom, x, y);
    });
    visitDrawer(function (x, y, atom) {
        if (atom === state['selectedAtom']) {
            ctx.fillStyle = isLight ? '#aab' : '#112';
            ctx.beginPath();
            ctx.ellipse(x, y, 45, 45, 0, 0, 2 * Math.PI);
            ctx.fill();
        }
        ctx.fillStyle = 'rgba(0, 0, 0, 0.35)';
        ctx.beginPath();
        ctx.ellipse(x + 4, y + 4, 29, 29, 0, 0, 2 * Math.PI);
        ctx.fill();
    }, function (x, y, bond) {
        if (bond === state['selectedBond']) {
            ctx.fillStyle = isLight ? '#aab' : '#223';
            ctx.save();
            ctx.translate(x, y);
            ctx.beginPath();
            roundRect(ctx, -40, -30, 80, 60, 20);
            ctx.fill();
            ctx.restore();
        }
        ctx.save();
        ctx.translate(x + 4, y + 4);
        drawBond(ctx, bond.includes('r'), bond.includes('k'), bond.includes('y'), 40, 'shadow');
        ctx.restore();
    });
    visitDrawer(function (x, y, atom) {
        drawAtom(ctx, atom, x, y);
    }, function (x, y, bond) {
        ctx.save();
        ctx.translate(x, y);
        drawBond(ctx, bond.includes('r'), bond.includes('k'), bond.includes('y'), 40);
        ctx.restore();
    });
    ctx.restore();
}

function redraw() {
    draw(canvas, state);
    draw(nextCanvas, nextState);
    const ctx = canvas.getContext('2d');
    ctx.save();
    ctx.globalAlpha = 0.1;
    ctx.fillStyle = 'black';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.globalCompositeOperation = 'lighter';
    ctx.drawImage(nextCanvas, 0, 0, canvas.width, canvas.height);
    ctx.restore();
}

window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', redraw);
redraw();

let mouseDown = false;
let erasing = false;
let mode = 'atom';
function removeBond(state, i0, j0, i1, j1) {
    const key = `${i0},${j0}:${i1},${j1}`;
    if (state[key])
        delete state[key];
}
function updateNextState() {
    let closestI = 9999;
    let closestJ = 9999;
    let closestDistance2 = 99999999;
    visit(function (i, j, x, y) {
        const distance2 = (x - mouseX) * (x - mouseX) + (y - mouseY) * (y - mouseY);
        if (distance2 < closestDistance2) {
            closestDistance2 = distance2;
            closestI = i;
            closestJ = j;
        }
    });
    const atomKey = `${closestI},${closestJ}`;
    if (!mouseDown) {
        nextState = Object.assign({}, state);
        mode = (!nextState[atomKey] || closestDistance2 < 30 * 30) ? 'atom' : 'bond';
    }
    if (closestDistance2 > 50 * 50) {
        closestDistance2 = 99999999;
        let closestAtom;
        let closestBond;
        visitDrawer(function (x, y, atom) {
            const distance2 = (x - mouseX) * (x - mouseX) + (y - mouseY) * (y - mouseY);
            if (distance2 < closestDistance2) {
                closestDistance2 = distance2;
                closestAtom = atom;
            }
        }, function (x, y, bond) {
            const distance2 = (x - mouseX) * (x - mouseX) + (y - mouseY) * (y - mouseY);
            if (distance2 < closestDistance2) {
                closestDistance2 = distance2;
                closestAtom = null;
                closestBond = bond;
            }
        });
        if (closestDistance2 < 50 * 50) {
            if (closestAtom)
                nextState['selectedAtom'] = closestAtom;
            else
                nextState['selectedBond'] = closestBond;
        }
        redraw();
        return;
    }
    let closestI0 = 9999;
    let closestJ0 = 9999;
    let closestI1 = 9999;
    let closestJ1 = 9999;
    let closestBondDistance2 = 99999999;
    visit(null, function (i0, j0, i1, j1, x, y, rotation) {
        if (mode === 'bond' && !nextState[atomKey])
            return;
        else if (i0 === closestI && j0 === closestJ) {
            if (!nextState[`${i1},${j1}`])
                return;
        } else if (i1 === closestI && j1 === closestJ) {
            if (!nextState[`${i0},${j0}`])
                return;
        } else
            return;
        const distance2 = (x - mouseX) * (x - mouseX) + (y - mouseY) * (y - mouseY);
        if (distance2 < closestBondDistance2) {
            closestBondDistance2 = distance2;
            closestI0 = i0;
            closestJ0 = j0;
            closestI1 = i1;
            closestJ1 = j1;
        }
    });
    const bondKey = `${closestI0},${closestJ0}:${closestI1},${closestJ1}`;
    if (!mouseDown) {
        if (mode === 'atom')
            erasing = nextState[atomKey] && nextState[atomKey] === nextState['selectedAtom'];
        else if (mode === 'bond')
            erasing = nextState[bondKey] && nextState[bondKey] === nextState['selectedBond'];
        else
            erasing = false;
    }
    if (mode === 'atom') {
        if (erasing) {
            delete nextState[atomKey];
            removeBond(nextState, closestI - 1, closestJ, closestI, closestJ);
            removeBond(nextState, closestI, closestJ - 1, closestI, closestJ);
            removeBond(nextState, closestI + 1, closestJ - 1, closestI, closestJ);
            removeBond(nextState, closestI, closestJ, closestI + 1, closestJ);
            removeBond(nextState, closestI, closestJ, closestI, closestJ + 1);
            removeBond(nextState, closestI, closestJ, closestI - 1, closestJ + 1);
        } else {
            if ((!nextState[atomKey] && closestBondDistance2 < 99999999) || (closestDistance2 > 29 * 29 && closestBondDistance2 < 20 * 20))
                nextState[bondKey] = nextState['selectedBond'];
            if (!nextState[atomKey] || closestDistance2 <= 29 * 29)
                nextState[atomKey] = nextState['selectedAtom'];
        }
    } else if (mode === 'bond') {
        if (erasing && closestBondDistance2 < 25 * 25)
            delete nextState[bondKey];
        else if (closestBondDistance2 < 15 * 15)
            nextState[bondKey] = nextState['selectedBond'];
    }
}
function sortedAtomPositions(state) {
    return Object.keys(state).map(function (a) {
        return a.split(',').map(function (n) { return parseInt(n, 10); });
    }).filter(function (a) {
        return a.length === 2;
    }).sort(function (a, b) {
        if (a[0] < b[0])
            return -1;
        if (a[0] > b[0])
            return 1;
        if (a[1] < b[1])
            return -1;
        if (a[1] > b[1])
            return 1;
        return 0;
    });
}
function stateToNumber(state) {
    const atomPositions = sortedAtomPositions(state);
    let cursor;
    let multiplier = 1n;
    let number = 0n;
    for (const position of atomPositions) {
        if (cursor && cursor[0] !== position[0]) {
            while (cursor[1] > position[1]) {
                multiplier *= 3n;
                cursor[1]--;
            }
            while (cursor[0] < position[0]) {
                number += multiplier * 2n;
                multiplier *= 3n;
                cursor[0]++;
            }
        }
        if (cursor && cursor[1] !== position[1]) {
            cursor[1]++;
            while (cursor[1] !== position[1]) {
                multiplier *= 3n;
                cursor[1]++;
            }
        }
        if (cursor) {
            number += multiplier;
            multiplier *= 3n;
        }
        const atom = state[`${position[0]},${position[1]}`];
        number += atomEncoding[atom] * multiplier;
        multiplier *= 15n;
        cursor = JSON.parse(JSON.stringify(position));
    }
    for (const position of atomPositions) {
        if (state[`${position[0]+1},${position[1]}`]) {
            number += bondEncoding(state[`${position[0]},${position[1]}:${position[0]+1},${position[1]}`]) * multiplier;
            multiplier *= 3n;
        }
        if (state[`${position[0]-1},${position[1]+1}`]) {
            number += bondEncoding(state[`${position[0]},${position[1]}:${position[0]-1},${position[1]+1}`]) * multiplier;
            multiplier *= 3n;
        }
        if (state[`${position[0]},${position[1]+1}`]) {
            number += bondEncoding(state[`${position[0]},${position[1]}:${position[0]},${position[1]+1}`]) * multiplier;
            multiplier *= 3n;
        }
    }
    return number;
}
function visitBondForValidation(state, result, stack, visited, p, u, v) {
    const bondNeighbor = [p[0] + u, p[1] + v];
    const bondKey = keyForBond(canonicalizeBond([p, bondNeighbor]));
    const bond = state[bondKey];
    if (!bond)
        return;
    const bondNeighborJSON = JSON.stringify(bondNeighbor);
    if (visited.has(bondNeighborJSON))
        return;
    visited.add(bondNeighborJSON);
    stack.push(bondNeighbor);

}
function validateState(state) {
    const result = {changed: true};
    if (state === prevState) {
        result.changed = false;
        return result;
    }
    prevState = Object.assign({}, state);
    const atomPositions = Object.keys(state).map(function (a) {
        return a.split(',').map(function (n) { return parseInt(n, 10); });
    }).filter(function (a) {
        return a.length === 2;
    });
    if (atomPositions.length === 0) {
        result.empty = true;
        return result;
    }
    const stack = [atomPositions[0]];
    const visited = new Set([JSON.stringify(atomPositions[0])]);
    while (stack.length > 0) {
        const p = stack.pop();
        visitBondForValidation(state, result, stack, visited, p, 1, 0);
        visitBondForValidation(state, result, stack, visited, p, 0, 1);
        visitBondForValidation(state, result, stack, visited, p, -1, 1);
        visitBondForValidation(state, result, stack, visited, p, -1, 0);
        visitBondForValidation(state, result, stack, visited, p, 0, -1);
        visitBondForValidation(state, result, stack, visited, p, 1, -1);
    }
    return result;
}
function triangular(index) {
    let n = 1;
    while (true) {
        if (index < n)
            return [ n, index ];
        index -= n;
        n++;
    }
}
function tetrahedral(index) {
    let n = 2;
    while (true) {
        if (index < n * (n - 1) / 2) {
            const [ a, b ] = triangular(index);
            return [ n, a, b ];
        }
        index -= n * (n - 1) / 2;
        n++;
    }
}
function stateForEnumerationIndex(index) {
    if (index < 0)
        throw 'index out of range';
    if (index < 15)
        return { '0,0': atomsByEncoding[index] };
    index -= 15;
    if (index < 120) {
        const [ a, b ] = triangular(index);
        return { '0,0': atomsByEncoding[a - 1], '1,0': atomsByEncoding[b], '0,0:1,0': 'n' };
    }
    index -= 120;
    if (index < 1800) {
        const b = index % 15;
        const [ a, c ] = triangular(Math.floor(index / 15));
        return {
            '0,0': atomsByEncoding[a - 1],
            '1,0': atomsByEncoding[b],
            '2,0': atomsByEncoding[c],
            '0,0:1,0': 'n',
            '1,0:2,0': 'n',
        };
    }
    index -= 1800;
    if (index < 3375) {
        const a = index % 15;
        index = Math.floor(index / 15);
        const b = index % 15;
        index = Math.floor(index / 15);
        const c = index % 15;
        return {
            '0,0': atomsByEncoding[a],
            '1,-1': atomsByEncoding[b],
            '1,0': atomsByEncoding[c],
            '1,-1:0,0': 'n',
            '1,-1:1,0': 'n',
        };
    }
    index -= 3375;
    if (index < 3375) {
        const a = index % 15;
        index = Math.floor(index / 15);
        const b = index % 15;
        index = Math.floor(index / 15);
        const c = index % 15;
        return {
            '0,0': atomsByEncoding[a],
            '1,-1': atomsByEncoding[b],
            '2,-1': atomsByEncoding[c],
            '1,-1:0,0': 'n',
            '1,-1:2,-1': 'n',
        };
    }
    index -= 3375;
    if (index < 225) {
        const a = index % 15;
        index = Math.floor(index / 15);
        const b = index % 15;
        return {
            '0,0': atomsByEncoding[a],
            '1,-1': atomsByEncoding[b],
            '1,0': atomsByEncoding[a],
            '1,-1:0,0': 'n',
            '1,-1:1,0': 'n',
            '0,0:1,0': 'n',
        };
    }
    index -= 225;
    if (index < 910) {
        const flip = index % 2;
        index = Math.floor(index / 2);
        const [ a, b, c ] = tetrahedral(index);
        return {
            '0,0': atomsByEncoding[a],
            '1,-1': flip ? atomsByEncoding[b] : atomsByEncoding[c],
            '1,0': flip ? atomsByEncoding[c] : atomsByEncoding[b],
            '1,-1:0,0': 'n',
            '1,-1:1,0': 'n',
            '0,0:1,0': 'n',
        };
    }
    index -= 910;
    if (index < 15) {
        return {
            '0,0': 'fire',
            '1,0': 'fire',
            '2,0': atomsByEncoding[index],
            '0,0:1,0': 'ryk',
            '1,0:2,0': 'n',
        };
    }
    index -= 15;
    if (index < 15) {
        return {
            '0,0': 'fire',
            '1,-1': 'fire',
            '1,0': atomsByEncoding[index],
            '1,-1:0,0': 'ryk',
            '1,-1:1,0': 'n',
        };
    }
    index -= 15;
    if (index < 15) {
        return {
            '0,0': atomsByEncoding[index],
            '1,-1': 'fire',
            '1,0': 'fire',
            '1,-1:0,0': 'n',
            '1,-1:1,0': 'ryk',
        };
    }
    index -= 15;
    if (index < 15) {
        return {
            '0,0': 'fire',
            '1,-1': 'fire',
            '2,-1': atomsByEncoding[index],
            '1,-1:0,0': 'ryk',
            '1,-1:2,-1': 'n',
        };
    }
    index -= 15;
    if (index < 15) {
        return {
            '0,0': atomsByEncoding[index],
            '1,-1': 'fire',
            '2,-1': 'fire',
            '1,-1:0,0': 'n',
            '1,-1:2,-1': 'ryk',
        };
    }
    index -= 15;
    if (index < 15) {
        return {
            '0,0': 'fire',
            '1,-1': atomsByEncoding[index],
            '1,0': 'fire',
            '1,-1:0,0': 'n',
            '1,-1:1,0': 'n',
            '0,0:1,0': 'ryk',
        };
    }
    index -= 15;
    switch (index) {
    case 0:
        return { '0,0': 'fire', '1,0': 'fire', '0,0:1,0': 'ryk' };
    case 1:
        return { '0,0': 'fire', '1,0': 'fire', '2,0': 'fire', '0,0:1,0': 'ryk', '1,0:2,0': 'ryk' };
    case 2:
        return { '0,0': 'fire', '1,-1': 'fire', '1,0': 'fire', '1,-1:0,0': 'ryk', '1,-1:1,0': 'ryk' };
    case 3:
        return { '0,0': 'fire', '1,-1': 'fire', '2,-1': 'fire', '1,-1:0,0': 'ryk', '1,-1:2,-1': 'ryk' };
    case 4:
        return { '0,0': 'fire', '1,-1': 'fire', '1,0': 'fire', '1,-1:0,0': 'ryk', '1,-1:1,0': 'ryk', '0,0:1,0': 'n' };
    case 5:
        return { '0,0': 'fire', '1,-1': 'fire', '1,0': 'fire', '1,-1:0,0': 'ryk', '1,-1:1,0': 'ryk', '0,0:1,0': 'ryk' };
    default:
        throw 'number out of range';
    }
}
async function updateDownload() {
    document.getElementById('permalink').onclick = () => {};
    const validationResult = validateState(state);
    if (!validationResult.changed) {
        return;
    } else if (validationResult.empty) {
        document.getElementById('clear').style.display = 'none';
        document.getElementById('permalink').style.display = 'none';
        document.getElementById('default').style.display = '';
        document.getElementById('error').textContent = '';
        document.getElementById('result').style.display = 'none';
        return;
    } else if (validationResult.error) {
        document.getElementById('clear').style.display = '';
        document.getElementById('permalink').style.display = 'none';
        document.getElementById('default').style.display = 'none';
        document.getElementById('error').textContent = validationResult.error;
        document.getElementById('result').style.display = 'none';
        return;
    } else {
        let response = await fetch(new Request('/api/v3/molecule-from-state', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(state),
        }));
        if (response.ok) {
            let data = await response.json();
            document.getElementById('permalink').onclick = async function (e) {
                await navigator.clipboard.writeText(`https://mol.fenhl.net/?m=${data.permalink}`);
            };
            if (data.appearances.length === 0) {
                const message = document.createElement('h1');
                message.setAttribute('class', 'muted');
                message.appendChild(document.createTextNode('unknown molecule'));
                message.addEventListener('click', async function (e) {
                    await navigator.clipboard.writeText(data.rustCode);
                });
                document.getElementById('result').replaceChildren(message);
            } else if (data.appearances.every(val => val.name === null)) {
                const message = document.createElement('h1');
                message.setAttribute('class', 'muted');
                message.appendChild(document.createTextNode('unnamed molecule'));
                message.addEventListener('click', async function (e) {
                    await navigator.clipboard.writeText(data.rustCode);
                });
                const appearances = document.createElement('p');
                data.appearances.forEach((val, idx) => {
                    if (idx > 0) {
                        const comma = document.createElement('span');
                        comma.setAttribute('class', 'muted');
                        comma.appendChild(document.createTextNode(', '));
                        appearances.appendChild(comma);
                    }
                    let puzzle = document.createTextNode(val.puzzle);
                    if (val.url !== null) {
                        puzzle = document.createElement('a');
                        puzzle.setAttribute('href', val.url);
                        puzzle.appendChild(document.createTextNode(val.puzzle));
                    }
                    appearances.appendChild(puzzle);
                    const inOut = document.createElement('em');
                    inOut.setAttribute('class', 'muted');
                    switch (val.inout) {
                        case 'Reagent': {
                            inOut.setAttribute('title', 'appears as reagent');
                            inOut.appendChild(document.createTextNode(' r'));
                            break;
                        }
                        case 'Product': {
                            inOut.setAttribute('title', 'appears as product');
                            inOut.appendChild(document.createTextNode(' p'));
                            break;
                        }
                        case 'Both': {
                            inOut.setAttribute('title', 'appears as both reagent and product');
                            inOut.appendChild(document.createTextNode(' rp'));
                            break;
                        }
                        default: {
                            throw 'unknown InOut kind';
                        }
                    }
                    appearances.appendChild(inOut);
                });
                document.getElementById('result').replaceChildren(message, appearances);
            } else {
                const firstName = data.appearances.find(val => val.name !== null).name;
                if (data.appearances.every(val => val.name === null || val.name === firstName)) {
                    const name = document.createElement('h1');
                    name.appendChild(document.createTextNode(firstName));
                    const appearances = document.createElement('p');
                    data.appearances.filter(val => val.name !== null).forEach((val, idx) => {
                        if (idx > 0) {
                            const comma = document.createElement('span');
                            comma.setAttribute('class', 'muted');
                            comma.appendChild(document.createTextNode(', '));
                            appearances.appendChild(comma);
                        }
                        let puzzle = document.createTextNode(val.puzzle);
                        if (val.url !== null) {
                            puzzle = document.createElement('a');
                            puzzle.setAttribute('href', val.url);
                            puzzle.appendChild(document.createTextNode(val.puzzle));
                        }
                        appearances.appendChild(puzzle);
                        const inOut = document.createElement('em');
                        inOut.setAttribute('class', 'muted');
                        switch (val.inout) {
                            case 'Reagent': {
                                inOut.setAttribute('title', 'appears as reagent');
                                inOut.appendChild(document.createTextNode(' r'));
                                break;
                            }
                            case 'Product': {
                                inOut.setAttribute('title', 'appears as product');
                                inOut.appendChild(document.createTextNode(' p'));
                                break;
                            }
                            case 'Both': {
                                inOut.setAttribute('title', 'appears as both reagent and product');
                                inOut.appendChild(document.createTextNode(' rp'));
                                break;
                            }
                            default: {
                                throw 'unknown InOut kind';
                            }
                        }
                        appearances.appendChild(inOut);
                    });
                    if (data.appearances.some(val => val.name === null)) {
                        const unnamedAppearances = document.createElement('p');
                        const prefix = document.createElement('span');
                        prefix.setAttribute('class', 'muted');
                        prefix.appendChild(document.createTextNode('unnamed: '));
                        unnamedAppearances.appendChild(prefix);
                        data.appearances.filter(val => val.name === null).forEach((val, idx) => {
                            if (idx > 0) {
                                const comma = document.createElement('span');
                                comma.setAttribute('class', 'muted');
                                comma.appendChild(document.createTextNode(', '));
                                unnamedAppearances.appendChild(comma);
                            }
                            let puzzle = document.createTextNode(val.puzzle);
                            if (val.url !== null) {
                                puzzle = document.createElement('a');
                                puzzle.setAttribute('href', val.url);
                                puzzle.appendChild(document.createTextNode(val.puzzle));
                            }
                            unnamedAppearances.appendChild(puzzle);
                            const inOut = document.createElement('em');
                            inOut.setAttribute('class', 'muted');
                            switch (val.inout) {
                                case 'Reagent': {
                                    inOut.setAttribute('title', 'appears as reagent');
                                    inOut.appendChild(document.createTextNode(' r'));
                                    break;
                                }
                                case 'Product': {
                                    inOut.setAttribute('title', 'appears as product');
                                    inOut.appendChild(document.createTextNode(' p'));
                                    break;
                                }
                                case 'Both': {
                                    inOut.setAttribute('title', 'appears as both reagent and product');
                                    inOut.appendChild(document.createTextNode(' rp'));
                                    break;
                                }
                                default: {
                                    throw 'unknown InOut kind';
                                }
                            }
                            unnamedAppearances.appendChild(inOut);
                        });
                        document.getElementById('result').replaceChildren(name, appearances, unnamedAppearances);
                    } else {
                        document.getElementById('result').replaceChildren(name, appearances);
                    }
                } else {
                    const message = document.createElement('h1');
                    const inner = document.createElement('em');
                    inner.appendChild(document.createTextNode('multiple names'));
                    message.appendChild(inner);
                    const names = data.appearances.map(val => val.name).filter((val, idx, array) => val !== null && array.indexOf(val) === idx);
                    const appearances = names.map((name) => {
                        const nameElt = document.createElement('p');
                        const prefix = document.createElement('span');
                        prefix.setAttribute('class', 'muted');
                        prefix.appendChild(document.createTextNode('as '));
                        nameElt.appendChild(prefix);
                        nameElt.appendChild(document.createTextNode(name));
                        const colon = document.createElement('span');
                        colon.setAttribute('class', 'muted');
                        colon.appendChild(document.createTextNode(': '));
                        nameElt.appendChild(colon);
                        data.appearances.filter(val => val.name === name).forEach((val, idx) => {
                            if (idx > 0) {
                                const comma = document.createElement('span');
                                comma.setAttribute('class', 'muted');
                                comma.appendChild(document.createTextNode(', '));
                                nameElt.appendChild(comma);
                            }
                            let puzzle = document.createTextNode(val.puzzle);
                            if (val.url !== null) {
                                puzzle = document.createElement('a');
                                puzzle.setAttribute('href', val.url);
                                puzzle.appendChild(document.createTextNode(val.puzzle));
                            }
                            nameElt.appendChild(puzzle);
                            const inOut = document.createElement('em');
                            inOut.setAttribute('class', 'muted');
                            switch (val.inout) {
                                case 'Reagent': {
                                    inOut.setAttribute('title', 'appears as reagent');
                                    inOut.appendChild(document.createTextNode(' r'));
                                    break;
                                }
                                case 'Product': {
                                    inOut.setAttribute('title', 'appears as product');
                                    inOut.appendChild(document.createTextNode(' p'));
                                    break;
                                }
                                case 'Both': {
                                    inOut.setAttribute('title', 'appears as both reagent and product');
                                    inOut.appendChild(document.createTextNode(' rp'));
                                    break;
                                }
                                default: {
                                    throw 'unknown InOut kind';
                                }
                            }
                            nameElt.appendChild(inOut);
                        });
                        return nameElt;
                    });
                    if (data.appearances.some(val => val.name === null)) {
                        const unnamedAppearances = document.createElement('p');
                        const prefix = document.createElement('span');
                        prefix.setAttribute('class', 'muted');
                        prefix.appendChild(document.createTextNode('unnamed: '));
                        unnamedAppearances.appendChild(prefix);
                        data.appearances.filter(val => val.name === null).forEach((val, idx) => {
                            if (idx > 0) {
                                const comma = document.createElement('span');
                                comma.setAttribute('class', 'muted');
                                comma.appendChild(document.createTextNode(', '));
                                unnamedAppearances.appendChild(comma);
                            }
                            let puzzle = document.createTextNode(val.puzzle);
                            if (val.url !== null) {
                                puzzle = document.createElement('a');
                                puzzle.setAttribute('href', val.url);
                                puzzle.appendChild(document.createTextNode(val.puzzle));
                            }
                            unnamedAppearances.appendChild(puzzle);
                            const inOut = document.createElement('em');
                            inOut.setAttribute('class', 'muted');
                            switch (val.inout) {
                                case 'Reagent': {
                                    inOut.setAttribute('title', 'appears as reagent');
                                    inOut.appendChild(document.createTextNode(' r'));
                                    break;
                                }
                                case 'Product': {
                                    inOut.setAttribute('title', 'appears as product');
                                    inOut.appendChild(document.createTextNode(' p'));
                                    break;
                                }
                                case 'Both': {
                                    inOut.setAttribute('title', 'appears as both reagent and product');
                                    inOut.appendChild(document.createTextNode(' rp'));
                                    break;
                                }
                                default: {
                                    throw 'unknown InOut kind';
                                }
                            }
                            unnamedAppearances.appendChild(inOut);
                        });
                        appearances.push(unnamedAppearances);
                    }
                    document.getElementById('result').replaceChildren(message, ...appearances);
                }
            }
            document.getElementById('clear').style.display = '';
            document.getElementById('permalink').style.display = '';
            document.getElementById('default').style.display = 'none';
            document.getElementById('error').textContent = '';
            document.getElementById('result').style.display = '';
        } else {
            document.getElementById('clear').style.display = '';
            document.getElementById('permalink').style.display = 'none';
            document.getElementById('default').style.display = 'none';
            document.getElementById('error').textContent = 'molecule lookup failed';
            document.getElementById('result').style.display = 'none';
        }
    }
}
function rotateCCW(point) {
    if (point.length !== 2)
        return point;
    const u = parseInt(point[0], 10);
    const v = parseInt(point[1], 10);
    return [-v, u + v];
}
function canonicalizeBond(bond) {
    if (bond.length !== 2)
        return bond;
    if (bond[0][0] === bond[1][0] - 1 && bond[0][1] === bond[1][1])
        return bond;
    if (bond[0][0] === bond[1][0] + 1 && bond[0][1] === bond[1][1] - 1)
        return bond;
    if (bond[0][0] === bond[1][0] && bond[0][1] === bond[1][1] - 1)
        return bond;
    return [bond[1], bond[0]];
}
function keyForBond(bond) {
    return bond.map(function (entry) { return entry.join(','); }).join(':');
}
function rotateStateCCW(state) {
    const rotated = {};
    for (const key of Object.keys(state))
        rotated[keyForBond(canonicalizeBond(key.split(':').map(function (entry) { return rotateCCW(entry.split(',')); })))] = state[key];
    return rotated;
}
window.addEventListener('mousemove', async function (e) {
    const r = canvas.getBoundingClientRect();
    mouseX = e.clientX - r.left;
    mouseY = e.clientY - r.top;
    updateNextState();
    redraw();
    if (mouseDown)
        await updateDownload();
});
canvas.addEventListener('contextmenu', function (e) {
    e.preventDefault();
});
canvas.addEventListener('mousedown', async function (e) {
    mouseDown = true;
    if (e.button === 2) {
        erasing = true;
        updateNextState();
    }
    state = nextState;
    redraw();
    await updateDownload();
});
window.addEventListener('mouseup', function (e) {
    mouseDown = false;
    updateNextState();
    redraw();
});

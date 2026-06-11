const atomStyle = {
    salt: { shadowStyle: 'black', fillStyle: '#eee', symbolStyle: '#ccc', symbol: [
        { type: 'arc', x: 0, y: 0, radius: 0.5, from: 0, to: 1 },
        { type: 'line', points: [ -0.5, 0, 0.5, 0 ] },
    ] },
    air: { shadowStyle: 'black', fillStyle: '#bef', symbolStyle: '#9cc', symbol: [
        { type: 'loop', points: [ 0, -0.45, 0.45, 0.32, -0.45, 0.32 ] },
        { type: 'line', points: [ -0.5, -0.05, 0.5, -0.05 ] },
    ] },
    fire: { shadowStyle: 'black', fillStyle: '#f45', symbolStyle: '#c34', symbol: [
        { type: 'loop', points: [ 0, -0.45, 0.45, 0.32, -0.45, 0.32 ] },
    ] },
    quicksilver: { shadowStyle: 'black', fillStyle: '#ddd', symbolStyle: '#bbb', symbol: [
        { type: 'arc', x: 0, y: -0.6, radius: 0.25, from: 0, to: 0.5 },
        { type: 'arc', x: 0, y: -0.05, radius: 0.3, from: 0, to: 1 },
        { type: 'line', points: [ 0, 0.25, 0, 0.65 ] },
        { type: 'line', points: [ -0.2, 0.45, 0.2, 0.45 ] },
    ] },
    water: { shadowStyle: 'black', fillStyle: '#0bf', symbolStyle: '#09c', symbol: [
        { type: 'loop', points: [ 0, 0.45, 0.45, -0.32, -0.45, -0.32 ] },
    ] },
    earth: { shadowStyle: 'black', fillStyle: '#6e4', symbolStyle: '#4c2', symbol: [
        { type: 'loop', points: [ 0, 0.45, 0.45, -0.32, -0.45, -0.32 ] },
        { type: 'line', points: [ -0.5, 0, 0.5, 0 ] },
    ] },
    lead: { shadowStyle: 'black', fillStyle: '#458', symbolStyle: '#67b', symbol: [
        { type: 'arc', x: 0.075, y: 0.2, radius: 0.3, from: 0.5, to: 1.3 },
        { type: 'line', points: [ -0.225, -0.5, -0.225, 0.2 ] },
        { type: 'line', points: [ -0.425, -0.3, -0.025, -0.3 ] },
    ] },
    tin: { shadowStyle: 'black', fillStyle: '#876', symbolStyle: '#a98', symbol: [
        { type: 'line', points: [ -0.4, 0.2, 0.55, 0.2 ] },
        { type: 'line', points: [ 0.25, -0.4, 0.25, 0.5 ] },
        { type: 'arc', x: -0.2, y: -0.05, radius: 0.25, from: 0.55, to: 1.25 },
    ] },
    iron: { shadowStyle: 'black', fillStyle: '#844', symbolStyle: '#b66', symbol: [
        { type: 'arc', x: -0.15, y: 0.15, radius: 0.3, from: 0, to: 1 },
        { type: 'line', points: [ 0.062, -0.062, 0.4, -0.4 ] },
        { type: 'line', points: [ 0, -0.4, 0.4, -0.4, 0.4, 0 ] },
    ] },
    copper: { shadowStyle: 'black', fillStyle: '#b74', symbolStyle: '#d96', symbol: [
        { type: 'arc', x: 0, y: -0.2, radius: 0.3, from: 0, to: 1 },
        { type: 'line', points: [ 0, 0.1, 0, 0.55 ] },
        { type: 'line', points: [ -0.2, 0.325, 0.2, 0.325 ] },
    ] },
    silver: { shadowStyle: 'black', fillStyle: '#334', symbolStyle: '#556', symbol: [
        { type: 'arc', x: 0, y: 0, radius: 0.5, from: 0.6, to: 0.4 },
        { type: 'arc', x: -0.6, y: 0, radius: 0.8, from: 0.9, to: 0.1 },
        // { type: 'line', points: [ 0.15, -0.5, 0.15, 0.5 ] },
    ] },
    gold: { shadowStyle: 'black', fillStyle: '#d92', symbolStyle: '#fb3', symbol: [
        { type: 'arc', x: 0, y: 0, radius: 0.5, from: 0, to: 1 },
        { type: 'arc', x: 0, y: 0, radius: 0.05, from: 0, to: 1 },
    ] },
    vitae: { shadowStyle: 'black', fillStyle: '#fcc', symbolStyle: '#d99', symbol: [
        { type: 'loop', points: [ 0, -0.45, 0.32, 0.1, -0.32, 0.1 ] },
        { type: 'line', points: [ 0, 0.1, 0, 0.55 ] },
        { type: 'line', points: [ -0.2, 0.325, 0.2, 0.325 ] },
    ] },
    // mors: { shadowStyle: 'black', fillStyle: '#444', symbolStyle: '#222', symbol: [
    mors: { shadowStyle: 'black', fillStyle: '#444', symbolStyle: '#666', symbol: [
        { type: 'loop', points: [ 0, 0.45, 0.32, -0.1, -0.32, -0.1 ] },
        { type: 'line', points: [ 0, -0.1, 0, -0.55 ] },
        { type: 'line', points: [ -0.2, -0.325, 0.2, -0.325 ] },
    ] },
    quintessence: { shadowStyle: 'black', fillStyle: '#546', symbolStyle: '#768', symbol: [
        { type: 'loop', points: [ 0, 0.5, 0.45, -0.27, -0.45, -0.27 ] },
        { type: 'line', points: [ -0.15, -0.27, 0, -0.5, 0.15, -0.27 ] },
        { type: 'line', points: [ 0.3, 0, 0.45, 0.27, 0.15, 0.27 ] },
        { type: 'line', points: [ -0.3, 0, -0.45, 0.27, -0.15, 0.27 ] },
    ] },
    repeat: { shadowStyle: 'black', fillStyle: '#555', symbolStyle: '#333', symbol: [
        { type: 'arc', x: 0, y: 0, radius: 0.05, from: 0, to: 1 },
        { type: 'arc', x: -0.4, y: 0, radius: 0.05, from: 0, to: 1 },
        { type: 'arc', x: 0.4, y: 0, radius: 0.05, from: 0, to: 1 },
    ] },
};

function drawAtom(ctx, atom, x, y) {
    ctx.fillStyle = atomStyle[atom].fillStyle;
    ctx.beginPath();
    ctx.ellipse(x, y, 29, 29, 0, 0, 2 * Math.PI);
    ctx.fill();
    ctx.save();
    ctx.translate(x, y);
    ctx.scale(29, 29);
    ctx.lineWidth = 3 / 29;
    ctx.strokeStyle = atomStyle[atom].symbolStyle;
    for (const s of atomStyle[atom].symbol) {
        if (s.type === 'arc') {
            ctx.beginPath();
            ctx.ellipse(s.x, s.y, s.radius, s.radius, 0, s.from * 2 * Math.PI, s.to * 2 * Math.PI);
            ctx.stroke();
        } else if (s.type == 'line') {
            ctx.beginPath();
            ctx.moveTo(s.points[0], s.points[1]);
            for (let i = 2; i < s.points.length; i += 2)
                ctx.lineTo(s.points[i], s.points[i + 1]);
            ctx.stroke();
        } else if (s.type == 'loop') {
            ctx.beginPath();
            ctx.moveTo(s.points[0], s.points[1]);
            for (let i = 2; i < s.points.length; i += 2)
                ctx.lineTo(s.points[i], s.points[i + 1]);
            ctx.closePath();
            ctx.stroke();
        }
    }
    ctx.restore();
}

function drawBond(ctx, red, black, yellow, length, shadow) {
    if (red || black || yellow) {
        if (yellow) {
            ctx.lineWidth = 3;
            ctx.strokeStyle = shadow === 'shadow' ? '#0008' : '#fc5';
            ctx.beginPath();
            ctx.moveTo(-length / 2, -5.5);
            ctx.bezierCurveTo(length / 3, -5.5, -length / 3, 5.5, length / 2, 5.5);
            ctx.stroke();
        }
        if (black) {
            ctx.lineWidth = 3;
            ctx.strokeStyle = shadow === 'shadow' ? '#0008' : '#aaa';
            ctx.beginPath();
            ctx.moveTo(-length / 2, 0);
            ctx.lineTo(length / 2, 0);
            ctx.stroke();
        }
        if (red) {
            ctx.lineWidth = 3;
            ctx.strokeStyle = shadow === 'shadow' ? '#0008' : '#f45';
            ctx.beginPath();
            ctx.moveTo(-length / 2, 5.5);
            ctx.bezierCurveTo(length / 3, 5.5, -length / 3, -5.5, length / 2, -5.5);
            ctx.stroke();
        }
    } else {
        ctx.fillStyle = shadow === 'shadow' ? '#0008' : 'white';
        ctx.fillRect(-length / 2, -5, length, 10);
    }
}

function drawProductAtom(pctx, atom, min_x, i, j, shadow) {
    const x = 45 + 82 * (i + 0.5 * j - 0.5 * min_x);
    const y = 45 + 71 * j;
    if (shadow) {
        pctx.fillStyle = atomStyle[atom].shadowStyle;
        pctx.beginPath();
        pctx.ellipse(x + shadow, y + shadow, 29, 29, 0, 0, 2 * Math.PI);
        pctx.fill();
        return;
    }
    drawAtom(pctx, atom, x, y);
}

function drawProductBond(pctx, red, black, yellow, min_x, i, j, rotation, shadow) {
    const x = 45 + 82 * (i + 0.5 * j - 0.5 * min_x);
    const y = 45 + 71 * j;
    pctx.save();
    pctx.translate(x, y);
    if (shadow)
        pctx.translate(4, 4);
    else
        pctx.translate(2, 2);
    pctx.rotate(rotation * 2 * Math.PI);
    pctx.translate(41, 0);
    if (shadow)
        drawBond(pctx, red, black, yellow, 82, 'shadow');
    else
        drawBond(pctx, red, black, yellow, 82);
    pctx.restore();
}

function drawProduct(id, minX, width, height, atoms, bonds) {
    const productCanvas = document.getElementById(id);
    productCanvas.width = width * window.devicePixelRatio;
    productCanvas.style.width = `${width}px`;
    productCanvas.height = height * window.devicePixelRatio;
    productCanvas.style.height = `${height}px`;
    const pctx = productCanvas.getContext('2d');
    pctx.scale(window.devicePixelRatio, window.devicePixelRatio);
    pctx.scale(0.75, 0.75);
    pctx.fillStyle = '#223';
    for (let shadow = 4; shadow >= 0; shadow -= 4) {
        for (const bond of bonds) {
            const dq = bond.end.q - bond.start.q;
            const dr = bond.end.r - bond.start.r;
            let rotation = 0;
            switch (dq) {
                case -1: {
                    switch (dr) {
                        case 0: {
                            rotation = 3;
                            break;
                        }
                        case 1: {
                            rotation = 2;
                            break;
                        }
                        default: {
                            throw 'quantum bond';
                        }
                    }
                    break;
                }
                case 0: {
                    switch (dr) {
                        case -1: {
                            rotation = 4;
                            break;
                        }
                        case 1: {
                            rotation = 1;
                            break;
                        }
                        default: {
                            throw 'quantum bond';
                        }
                    }
                    break;
                }
                case 1: {
                    switch (dr) {
                        case -1: {
                            rotation = 5;
                            break;
                        }
                        case 0: {
                            rotation = 0;
                            break;
                        }
                        default: {
                            throw 'quantum bond';
                        }
                    }
                    break;
                }
                default: {
                    throw 'quantum bond';
                }
            }
            drawProductBond(pctx, bond.red, bond.black, bond.yellow, minX, bond.start.q, bond.start.r, rotation/6, shadow);
        }
        if (!shadow) {
            for (const atom of atoms) {
                drawProductAtom(pctx, atom.kind, minX, atom.q, atom.r, 2);
            }
        }
        for (const atom of atoms) {
            drawProductAtom(pctx, atom.kind, minX, atom.q, atom.r, shadow);
        }
    }
}

function drawParams(atoms, bonds) {
    const minX = atoms.length == 0 ? 0 : atoms.map((atom) => 2 * atom.q + atom.r).reduce((a, b) => Math.min(a, b));
    let width = (atoms.length == 0 ? 0 : atoms.map((atom) => 2 * atom.q + atom.r + 2).reduce((a, b) => Math.max(a, b))) - minX;
    let height = atoms.length == 0 ? 0 : atoms.map((atom) => atom.r + 1).reduce((a, b) => Math.max(a, b));
    width = (41 * width + 10) * 3 / 4;
    height = (71 * height + 20) * 3 / 4;
    return [minX, width, height, atoms, bonds];
}

function stick(string) {
    return drawParams(
        [...string.entries().map(([idx, kind]) => { return {kind: kind, q: 0, r: idx}; })],
        [...Array(string.length - 1).keys()].map((idx) => { return {start: {q: 0, r: idx}, end: {q: 0, r: idx + 1}, red: false, black: false, yellow: false}; }),
    );
}

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
function drawParamsFromState(state) {
    const atoms = [];
    const bonds = [];
    for (const [key, value] of Object.entries(state)) {
        if (key.indexOf(':') >= 0) {
            const [start, end] = key.split(':');
            const [sq, sr] = start.split(',').map((coord) => parseInt(coord));
            const [eq, er] = end.split(',').map((coord) => parseInt(coord));
            bonds.push({start: {q: sq, r: sr}, end: {q: eq, r: er}, red: value.indexOf('r') >= 0, black: value.indexOf('k') >= 0, yellow: value.indexOf('y') >= 0});
        } else {
            const [q, r] = key.split(',').map((coord) => parseInt(coord));
            atoms.push({kind: value, q, r});
        }
    }
    const minQ = atoms.map((atom) => atom.q).reduce((a, b) => Math.min(a, b));
    const minR = atoms.map((atom) => atom.r).reduce((a, b) => Math.min(a, b));
    return drawParams(
        atoms.map((atom) => { return {kind: atom.kind, q: atom.q - minQ, r: atom.r - minR}; }),
        bonds.map((bond) => { return {start: {q: bond.start.q - minQ, r: bond.start.r - minR}, end: {q: bond.end.q - minQ, r: bond.end.r - minR}, red: bond.red, black: bond.black, yellow: bond.yellow}; }),
    );
}

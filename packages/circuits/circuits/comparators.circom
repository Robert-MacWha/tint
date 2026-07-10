pragma circom 2.2.3;

// BBF hook: extracts bit i from in. Shr on Montgomery-form field elements
// is not implemented in circom-witness-rs's graph optimizer, so the bit
// extraction must go through a blackbox function.
function bbf_bit(in, i) {
    return (in >> i) & 1;
}

// Inlined from circomlib/bitify.circom to avoid that file's
// `include "comparators.circom"` pulling in the upstream IsZero and
// creating a duplicate-symbol conflict with the BBF version below.
template Num2Bits(n) {
    signal input in;
    signal output out[n];
    var lc1=0;

    var e2=1;
    for (var i = 0; i<n; i++) {
        out[i] <-- bbf_bit(in, i);
        out[i] * (out[i] -1 ) === 0;
        lc1 += out[i] * e2;
        e2 = e2+e2;
    }

    lc1 === in;
}

// BBF hook for circom-witness-rs: defers the conditional inverse computation
// to a registered blackbox function at witness-generation time, avoiding the
// Fr_isTrue-on-dynamic-value panic that the inline ternary would cause.
function bbf_inv(x) {
    return x != 0 ? 1/x : 0;
}

template IsZero() {
    signal input in;
    signal output out;

    signal inv;

    inv <-- bbf_inv(in);

    out <== -in*inv +1;
    in*out === 0;
}

template IsEqual() {
    signal input in[2];
    signal output out;

    component isz = IsZero();

    in[1] - in[0] ==> isz.in;

    isz.out ==> out;
}

template LessThan(n) {
    assert(n <= 252);
    signal input in[2];
    signal output out;

    component n2b = Num2Bits(n+1);

    n2b.in <== in[0]+ (1<<n) - in[1];

    out <== 1-n2b.out[n];
}

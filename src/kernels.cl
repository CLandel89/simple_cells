/*
Looks up 2 bits from the table (-> Table in automata.rs).
*/
inline uchar lookup (__constant uchar *table, ushort env) {
    return (table[env/4] >> ((env%4)*2)) & 3;
}

/*
Calculates a new byte from the environment of 9 bytes.
In other words: New 8×1 slice from a 10×3 slice (with 42 bits ignored).
(The function parameters are ushort for internal convenience).
*/
inline uchar work_byte (
        ushort TL, ushort TM, ushort TR,
        ushort ML, ushort MM, ushort MR,
        ushort BL, ushort BM, ushort BR,
        __constant uchar *table
)
{
    uchar result = 0;
    ushort env_0 =
        ((TL>>7)<<0) | ((TM&7)<<1) |
        ((ML>>7)<<4) | ((MM&7)<<5) |
        ((BL>>7)<<8) | ((BM&7)<<9) ;
    result |= lookup(table, env_0) << 0;
    ushort env_2 = ((TM>>1)&15) | (((MM>>1)&15)<<4) | (((BM>>1)&15)<<8);
    result |= lookup(table, env_2) << 2;
    ushort env_4 = ((TM>>3)&15) | (((MM>>3)&15)<<4) | (((BM>>3)&15)<<8);
    result |= lookup(table, env_4) << 4;
    ushort env_6 =
        ((TM>>5)<<0) | ((TR&1)<<3) |
        ((MM>>5)<<4) | ((MR&1)<<7) |
        ((BM>>5)<<8) | ((BR&1)<<11) ;
    result |= lookup(table, env_6) << 6;
    return result;
}

/*
Plays Game Of Life or so in the row specified by y.
*/
__kernel void play (
        uint w,
        uint h,
        __global uchar *source,
        __global uchar *target,
        __constant uchar *table
)
{
    int y = get_global_id(0);
    const size_t w8 = w%8 ? w/8+1 : w/8;
    // in case there are bits in the byte of the row (y) that need to be cleared
    uchar cutoff = 0;
    if (w%8 != 0) {
        for (int ci=w%8; ci<8; ci++)
            cutoff |= 1 << ci;
    }
    // source and target field operations ("t","m","b": top, mid and bottom rows)
    #define gt(x8) source[(y-1)*w8 + x8]
    #define gm(x8) source[y*w8 + x8]
    #define gb(x8) source[(y+1)*w8 + x8]
    #define sm(x8, v) target[y*w8 + x8] = v
    // top row
    if (y == 0) {
        // top left corner
        sm(0, work_byte(
            0, 0, 0,
            0, gm(0), gm(1),
            0, gb(0), gb(1),
            table
        ));
        // top stripe
        for (size_t x8=1; x8<w8-1; x8++) {
            sm(x8, work_byte(
                0, 0, 0,
                gm(x8-1), gm(x8), gm(x8+1),
                gb(x8-1), gb(x8), gb(x8+1),
                table
            ));
        }
        // top right corner
        sm(
            w8 - 1,
            work_byte(
                0, 0, 0,
                gm(w8-2), gm(w8-1), 0,
                gb(w8-2), gb(w8-1), 0,
                table
            ) & ~cutoff
        );
    }
    // mid row
    else if (y < h-1) {
        // left edge
        sm(0, work_byte(
            0, gt(0), gt(1),
            0, gm(0), gm(1),
            0, gb(0), gb(1),
            table
        ));
        // mid
        for (size_t x8=1; x8<w8-1; x8++) {
            sm(x8, work_byte(
                gt(x8-1), gt(x8), gt(x8+1),
                gm(x8-1), gm(x8), gm(x8+1),
                gb(x8-1), gb(x8), gb(x8+1),
                table
            ));
        }
        // right edge
        sm(
            w8 - 1,
            work_byte(
                gt(w8-2), gt(w8-1), 0,
                gm(w8-2), gm(w8-1), 0,
                gb(w8-2), gb(w8-1), 0,
                table
            ) & ~cutoff
        );
    }
    // bottom row
    else /*y == h-1*/ {
        // bottom left corner
        sm(0, work_byte(
            0, gt(0), gt(1),
            0, gm(0), gm(1),
            0, 0, 0,
            table
        ));
        // bottom stripe
        for (size_t x8=1; x8<w8-1; x8++) {
            sm(x8, work_byte(
                gt(x8-1), gt(x8), gt(x8+1),
                gm(x8-1), gm(x8), gm(x8+1),
                0, 0, 0,
                table
            ));
        }
        // bottom right corner
        sm(
            w8 - 1,
            work_byte(
                gt(w8-2), gt(w8-1), 0,
                gm(w8-2), gm(w8-1), 0,
                0, 0, 0,
                table
            ) & ~cutoff
        );
    }
    #undef gt
    #undef gm
    #undef gb
    #undef sm
}
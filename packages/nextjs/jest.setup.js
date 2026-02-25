global.TextEncoder = require('util').TextEncoder;
global.TextDecoder = require('util').TextDecoder;

// WASMモジュールのモック設定
jest.mock('../mpc-algebra-wasm/pkg-web', () => ({
    __esModule: true,
    default: jest.fn(),
    init: jest.fn().mockResolvedValue(undefined),
    encrypt_and_share: jest.fn().mockImplementation(async (input) => ({
        shares: [
            {
                node_id: 'node1',
                encrypted_share: 'encrypted_share_1',
                ephemeral_public_key: 'ephemeral_key_1',
            },
            {
                node_id: 'node2',
                encrypted_share: 'encrypted_share_2',
                ephemeral_public_key: 'ephemeral_key_2',
            },
        ],
        public_input: {
            pedersen_param: 'test_param',
            player_commitment: ['commitment1'],
        },
    })),
}));

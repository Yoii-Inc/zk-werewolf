require('dotenv').config({ path: '.env.local' });

module.exports = {
    preset: 'ts-jest',
    testEnvironment: 'node',
    moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx', 'json', 'node'],
    roots: ['<rootDir>/__tests__'],
    transform: {
        '^.+\\.tsx?$': 'ts-jest'
    },
    testMatch: [
        '**/__tests__/**/*.test.ts',
        '**/__tests__/**/*.spec.ts'
    ],
    moduleDirectories: ['node_modules', '<rootDir>'],
    moduleNameMapper: {
        '^~~/(.*)$': '<rootDir>/$1',
        '^\\.\\.\\/\\.\\.\\/\\.\\.\\/mpc-algebra-wasm\\/pkg-web\\/mpc_algebra_wasm$':
            '<rootDir>/../mpc-algebra-wasm/pkg-node/mpc_algebra_wasm'
    },
}

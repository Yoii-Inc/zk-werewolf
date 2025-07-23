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
}

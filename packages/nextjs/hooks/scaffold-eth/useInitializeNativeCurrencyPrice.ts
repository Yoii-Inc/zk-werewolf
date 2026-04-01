/**
 * Disabled: mainnet RPC connection is not needed for this project.
 * Price fetching via Uniswap requires mainnet access which causes 403 errors
 * with the default Alchemy API key.
 */
export const useInitializeNativeCurrencyPrice = () => {
  // noop
};

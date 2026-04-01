import { ChainWithAttributes, getAlchemyHttpUrl } from "./networks";
import { CurrencyAmount, Token } from "@uniswap/sdk-core";
import { Pair, Route } from "@uniswap/v2-sdk";
import { Address, createPublicClient, fallback, http, parseAbi } from "viem";
import { mainnet, sepolia } from "viem/chains";

const alchemyHttpUrl = getAlchemyHttpUrl(mainnet.id);
const rpcFallbacks = alchemyHttpUrl ? [http(alchemyHttpUrl), http()] : [http()];
const publicClient = createPublicClient({
  chain: mainnet,
  transport: fallback(rpcFallbacks),
});

const sepoliaAlchemyHttpUrl = getAlchemyHttpUrl(sepolia.id);
const sepoliaRpcFallbacks = sepoliaAlchemyHttpUrl ? [http(sepoliaAlchemyHttpUrl), http()] : [http()];
const sepoliaPublicClient = createPublicClient({
  chain: sepolia,
  transport: fallback(sepoliaRpcFallbacks),
});

const ABI = parseAbi([
  "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
  "function token0() external view returns (address)",
  "function token1() external view returns (address)",
]);

const CHAINLINK_AGGREGATOR_V3_ABI = parseAbi([
  "function decimals() view returns (uint8)",
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
]);
const SEPOLIA_ETH_USD_FEED = "0x694AA1769357215DE4FAC081bf1f309aDC325306" as Address;

const getChainClient = (targetNetwork: ChainWithAttributes) => {
  if (targetNetwork.id === sepolia.id) {
    return sepoliaPublicClient;
  }
  return publicClient;
};

export const fetchPriceFromUniswap = async (targetNetwork: ChainWithAttributes): Promise<number> => {
  if (
    targetNetwork.nativeCurrency.symbol !== "ETH" &&
    targetNetwork.nativeCurrency.symbol !== "SEP" &&
    !targetNetwork.nativeCurrencyTokenAddress
  ) {
    return 0;
  }

  // Sepolia does not have a reliable Uniswap V2 ETH/DAI pair for this flow.
  // Use Chainlink ETH/USD on Sepolia so we can keep price fetch on the same network.
  if (targetNetwork.id === sepolia.id) {
    try {
      const [answer, decimals] = await Promise.all([
        sepoliaPublicClient.readContract({
          address: SEPOLIA_ETH_USD_FEED,
          abi: CHAINLINK_AGGREGATOR_V3_ABI,
          functionName: "latestRoundData",
        }),
        sepoliaPublicClient.readContract({
          address: SEPOLIA_ETH_USD_FEED,
          abi: CHAINLINK_AGGREGATOR_V3_ABI,
          functionName: "decimals",
        }),
      ]);

      const price = Number(answer[1]) / 10 ** Number(decimals);
      return Number.isFinite(price) && price > 0 ? price : 0;
    } catch (error) {
      console.error(
        `useNativeCurrencyPrice - Error fetching ${targetNetwork.nativeCurrency.symbol} price on Sepolia: `,
        error,
      );
      return 0;
    }
  }

  try {
    const client = getChainClient(targetNetwork);
    const DAI = new Token(1, "0x6B175474E89094C44Da98b954EedeAC495271d0F", 18);
    const TOKEN = new Token(
      1,
      targetNetwork.nativeCurrencyTokenAddress || "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
      18,
    );
    const pairAddress = Pair.getAddress(TOKEN, DAI) as Address;

    const wagmiConfig = {
      address: pairAddress,
      abi: ABI,
    };

    const reserves = await client.readContract({
      ...wagmiConfig,
      functionName: "getReserves",
    });

    const token0Address = await client.readContract({
      ...wagmiConfig,
      functionName: "token0",
    });

    const token1Address = await client.readContract({
      ...wagmiConfig,
      functionName: "token1",
    });
    const token0 = [TOKEN, DAI].find(token => token.address === token0Address) as Token;
    const token1 = [TOKEN, DAI].find(token => token.address === token1Address) as Token;
    const pair = new Pair(
      CurrencyAmount.fromRawAmount(token0, reserves[0].toString()),
      CurrencyAmount.fromRawAmount(token1, reserves[1].toString()),
    );
    const route = new Route([pair], TOKEN, DAI);
    const price = parseFloat(route.midPrice.toSignificant(6));
    return price;
  } catch (error) {
    console.error(
      `useNativeCurrencyPrice - Error fetching ${targetNetwork.nativeCurrency.symbol} price from Uniswap: `,
      error,
    );
    return 0;
  }
};

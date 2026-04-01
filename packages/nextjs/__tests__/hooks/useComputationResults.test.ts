import { resolveDivinationWerewolfFlag } from "~~/hooks/useComputationResults";

describe("resolveDivinationWerewolfFlag", () => {
  test("returns false for not-werewolf plaintext", () => {
    const decrypted = {
      x: [["0", "0", "0", "0"], null],
      y: [["12436184717236109307", "3962172157175319849", "7381016538464732718", "1011752739694698287"], null],
      _params: null,
    };

    expect(resolveDivinationWerewolfFlag(decrypted)).toBe(false);
  });

  test("returns true for werewolf plaintext", () => {
    const decrypted = {
      x: [["15389767686415328915", "4532183014000888185", "6625844415766270035", "470379343721047487"], null],
      y: [["10215293119099184011", "9361858917463510870", "15793394060027790616", "2556078677302762916"], null],
      _params: null,
    };

    expect(resolveDivinationWerewolfFlag(decrypted)).toBe(true);
  });

  test("accepts bigint limbs without throwing", () => {
    const decrypted = {
      x: [[15389767686415328915n, 4532183014000888185n, 6625844415766270035n, 470379343721047487n], null],
      y: [[10215293119099184011n, 9361858917463510870n, 15793394060027790616n, 2556078677302762916n], null],
      _params: null,
    };

    expect(resolveDivinationWerewolfFlag(decrypted)).toBe(true);
  });

  test("returns null for unexpected plaintext", () => {
    const decrypted = {
      x: [["1", "2", "3", "4"], null],
      y: [["5", "6", "7", "8"], null],
      _params: null,
    };

    expect(resolveDivinationWerewolfFlag(decrypted)).toBeNull();
  });
});

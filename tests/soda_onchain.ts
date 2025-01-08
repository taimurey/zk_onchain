import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ZkOnchain } from "../target/types/zk_onchain";
import { PublicKey } from "@solana/web3.js";

describe("zk_onchain", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const owner = anchor.Wallet.local().payer;
  const program = anchor.workspace.ZkOnchain as Program<ZkOnchain>;

  it("Is initialized!", async () => {

    const ConfigAuthority = new PublicKey("Bn6jUQPC48meSkE5nZ8G8yWyxsuoiGwQwyX127nVmWWZ");

    // Add your test here.
    const config_authority = await getConfigAddress(
      ConfigAuthority,
      program.programId
    );


    // const tx = await program.methods.initializeUserVault(

    // ).accounts({
    //   payer: owner.publicKey,
    //   selfProgram: program.programId,
    //   serviceSigner: owner.publicKey,
    //   currentAuthority: owner.publicKey,
    //   config: config_authority
    // })
    // console.log("Your transaction signature", tx);
  });
});

export const VAULT_CONFIG_SEED = Buffer.from(
  anchor.utils.bytes.utf8.encode("vault-config")
);


export async function getConfigAddress(
  pool: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  const [address, bump] = await PublicKey.findProgramAddress(
    [VAULT_CONFIG_SEED, pool.toBuffer()],
    programId
  );
  return address;
}
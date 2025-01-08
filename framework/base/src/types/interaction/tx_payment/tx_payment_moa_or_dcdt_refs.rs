use crate::types::{BigUint, MoaOrDcdtTokenPaymentRefs, ManagedAddress, TxFrom, TxToSpecified};

use super::{Moa, FullPaymentData, FunctionCall, TxEnv, TxPayment};

impl<'a, Env> TxPayment<Env> for MoaOrDcdtTokenPaymentRefs<'a, Env::Api>
where
    Env: TxEnv,
{
    fn is_no_payment(&self, _env: &Env) -> bool {
        self.is_empty()
    }

    fn perform_transfer_execute(
        self,
        env: &Env,
        to: &ManagedAddress<Env::Api>,
        gas_limit: u64,
        fc: FunctionCall<Env::Api>,
    ) {
        self.map_moa_or_dcdt(
            fc,
            |fc, amount| Moa(amount).perform_transfer_execute(env, to, gas_limit, fc),
            |fc, dcdt_payment| dcdt_payment.perform_transfer_execute(env, to, gas_limit, fc),
        )
    }

    fn with_normalized<From, To, F, R>(
        self,
        env: &Env,
        from: &From,
        to: To,
        fc: FunctionCall<Env::Api>,
        f: F,
    ) -> R
    where
        From: TxFrom<Env>,
        To: TxToSpecified<Env>,
        F: FnOnce(&ManagedAddress<Env::Api>, &BigUint<Env::Api>, FunctionCall<Env::Api>) -> R,
    {
        self.map_moa_or_dcdt(
            (to, fc, f),
            |(to, fc, f), amount| Moa(amount).with_normalized(env, from, to, fc, f),
            |(to, fc, f), dcdt_payment| dcdt_payment.with_normalized(env, from, to, fc, f),
        )
    }

    fn into_full_payment_data(self, env: &Env) -> FullPaymentData<Env::Api> {
        self.map_moa_or_dcdt(
            (),
            |(), amount| TxPayment::<Env>::into_full_payment_data(Moa(amount), env),
            |(), dcdt_payment| TxPayment::<Env>::into_full_payment_data(dcdt_payment, env),
        )
    }
}
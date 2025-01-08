use crate::{
    api::CallTypeApi,
    types::{
        heap::H256, BigUint, CodeMetadata, MoaOrDcdtTokenIdentifier, MoaOrDcdtTokenPayment,
        MoaOrDcdtTokenPaymentRefs, MoaOrMultiDcdtPayment, DcdtTokenPayment, DcdtTokenPaymentRefs,
        ManagedAddress, ManagedBuffer, ManagedOption, ManagedVec, MultiDcdtPayment,
        TokenIdentifier,
    },
};

use dharitri_sc_codec::TopEncodeMulti;

use super::{
    AnnotatedValue, Code, ContractCallBase, ContractCallNoPayment, ContractCallWithMoa,
    ContractDeploy, DeployCall, Moa, MoaPayment, ExplicitGas, FromSource, FunctionCall,
    ManagedArgBuffer, OriginalResultMarker, RHList, RHListAppendNoRet, RHListAppendRet, RHListItem,
    TxCodeSource, TxCodeValue, TxData, TxDataFunctionCall, TxMoaValue, TxEnv,
    TxEnvMockDeployAddress, TxEnvWithTxHash, TxFrom, TxFromSourceValue, TxFromSpecified, TxGas,
    TxGasValue, TxPayment, TxPaymentMoaOnly, TxProxyTrait, TxResultHandler, TxScEnv, TxTo,
    TxToSpecified, UpgradeCall, UNSPECIFIED_GAS_LIMIT,
};

/// Universal representation of a blockchain transaction.
///
/// Uses 7 generic type arguments to encode all aspects of the transaction.
///
/// It is future-like, does nothing by itself, it needs a specialized method call to actually run or send it.
///
/// Rationale: https://twitter.com/andreimmarinica/status/1777157322155966601
#[must_use]
pub struct Tx<Env, From, To, Payment, Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    pub env: Env,
    pub from: From,
    pub to: To,
    pub payment: Payment,
    pub gas: Gas,
    pub data: Data,
    pub result_handler: RH,
}

impl<Env, From, To, Payment, Gas, Data, RH> Tx<Env, From, To, Payment, Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxDataFunctionCall<Env>,
    RH: TxResultHandler<Env>,
{
    /// Converts object to a Dharitri transaction data field string.
    pub fn to_call_data_string(&self) -> ManagedBuffer<Env::Api> {
        self.data.to_call_data_string()
    }
}

pub type TxBaseWithEnv<Env> = Tx<Env, (), (), (), (), (), ()>;

impl<Env> TxBaseWithEnv<Env>
where
    Env: TxEnv,
{
    /// Constructor, needs to take an environment object.
    #[inline]
    pub fn new_with_env(env: Env) -> Self {
        Tx {
            env,
            from: (),
            to: (),
            payment: (),
            gas: (),
            data: (),
            result_handler: (),
        }
    }
}

impl<Env, To, Payment, Gas, Data, RH> Tx<Env, (), To, Payment, Gas, Data, RH>
where
    Env: TxEnv,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Specifies transaction sender.
    pub fn from<From>(self, from: From) -> Tx<Env, From, To, Payment, Gas, Data, RH>
    where
        From: TxFrom<Env>,
    {
        Tx {
            env: self.env,
            from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, Payment, Gas, Data, RH> Tx<Env, From, (), Payment, Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Specifies the recipient of the transaction.
    ///
    /// Allows argument to also be `()`.
    pub fn to<To>(self, to: To) -> Tx<Env, From, To, Payment, Gas, Data, RH>
    where
        To: TxTo<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to,
            payment: self.payment,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, To, Gas, Data, RH> Tx<Env, From, To, (), Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Adds any payment to a transaction, if no payment has been added before.
    pub fn payment<Payment>(self, payment: Payment) -> Tx<Env, From, To, Payment, Gas, Data, RH>
    where
        Payment: TxPayment<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler,
        }
    }

    /// Adds MOA value to a transaction.
    ///
    /// Accepts any type that can represent and MOA amount: BigUint, &BigUint, etc.
    pub fn moa<MoaValue>(
        self,
        moa_value: MoaValue,
    ) -> Tx<Env, From, To, Moa<MoaValue>, Gas, Data, RH>
    where
        MoaValue: TxMoaValue<Env>,
    {
        self.payment(Moa(moa_value))
    }

    /// Backwards compatibility. Use method `moa` instead.
    pub fn with_moa_transfer(
        self,
        moa_amount: BigUint<Env::Api>,
    ) -> Tx<Env, From, To, MoaPayment<Env::Api>, Gas, Data, RH> {
        self.moa(moa_amount)
    }

    /// Adds the first single, owned DCDT token payment to a transaction.
    ///
    /// Since this is the first DCDT payment, a single payment tx is produced.
    ///
    /// Can subsequently be called again for multiple payments.
    pub fn dcdt<P: Into<DcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, DcdtTokenPayment<Env::Api>, Gas, Data, RH> {
        self.payment(payment.into())
    }

    /// Sets a single token payment, with the token identifier and amount kept as references.
    ///
    /// This is handy whem we only want one DCDT transfer and we want to avoid unnecessary object clones.
    pub fn single_dcdt<'a>(
        self,
        token_identifier: &'a TokenIdentifier<Env::Api>,
        token_nonce: u64,
        amount: &'a BigUint<Env::Api>,
    ) -> Tx<Env, From, To, DcdtTokenPaymentRefs<'a, Env::Api>, Gas, Data, RH> {
        self.payment(DcdtTokenPaymentRefs {
            token_identifier,
            token_nonce,
            amount,
        })
    }

    /// Syntactic sugar for `self.payment(MoaOrDcdtTokenPaymentRefs::new(...)`. Takes references.
    pub fn moa_or_single_dcdt<'a>(
        self,
        token_identifier: &'a MoaOrDcdtTokenIdentifier<Env::Api>,
        token_nonce: u64,
        amount: &'a BigUint<Env::Api>,
    ) -> Tx<Env, From, To, MoaOrDcdtTokenPaymentRefs<'a, Env::Api>, Gas, Data, RH> {
        self.payment(MoaOrDcdtTokenPaymentRefs::new(
            token_identifier,
            token_nonce,
            amount,
        ))
    }

    /// Sets a collection of DCDT transfers as the payment of the transaction.
    ///
    /// Can be formed from single DCDT payments, but the result will always be a collection.
    ///
    /// Always converts the argument into an owned collection of DCDT payments. For work with references, use `.payment(&p)` instead.
    pub fn multi_dcdt<IntoMulti>(
        self,
        payments: IntoMulti,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH>
    where
        IntoMulti: Into<MultiDcdtPayment<Env::Api>>,
    {
        self.payment(payments.into())
    }

    /// Backwards compatibility.
    pub fn with_dcdt_transfer<P: Into<DcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.payment(MultiDcdtPayment::new())
            .with_dcdt_transfer(payment)
    }

    /// Backwards compatibility.
    pub fn with_multi_token_transfer(
        self,
        payments: MultiDcdtPayment<Env::Api>,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.multi_dcdt(payments)
    }

    /// Backwards compatibility.
    pub fn with_moa_or_single_dcdt_transfer<P: Into<MoaOrDcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MoaOrDcdtTokenPayment<Env::Api>, Gas, Data, RH> {
        self.payment(payment.into())
    }

    /// Converts argument to `MoaOrMultiDcdtPayment`, then sets it as payment.
    ///
    /// In most cases, `payment` should be used instead.
    pub fn moa_or_multi_dcdt<P: Into<MoaOrMultiDcdtPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MoaOrMultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.payment(payment.into())
    }
}

impl<Env, From, To, Gas, Data, RH> Tx<Env, From, To, DcdtTokenPayment<Env::Api>, Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Adds the second DCDT token transfer to a contract call.
    ///
    /// Can be called multiple times on the same call.
    ///
    /// When the Tx already contains a single (owned) DCDT payment,
    /// adding the second one will convert it to a list.
    pub fn dcdt<P: Into<DcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        let mut payments = ManagedVec::new();
        payments.push(self.payment);
        payments.push(payment.into());
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: payments,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, To, Gas, Data, RH> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Adds a single DCDT token transfer to a contract call.
    ///
    /// Can be called multiple times on the same call.
    pub fn dcdt<P: Into<DcdtTokenPayment<Env::Api>>>(
        mut self,
        payment: P,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.payment.push(payment.into());
        self
    }

    /// When the Tx already contains an owned collection of DCDT payments,
    /// calling `multi_dcdt` is equivalent to `dcdt`, it just adds another payment to the list.
    ///
    /// Can be called multiple times.
    pub fn multi_dcdt<P: Into<DcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.dcdt(payment)
    }

    /// Backwards compatibility.
    pub fn with_dcdt_transfer<P: Into<DcdtTokenPayment<Env::Api>>>(
        self,
        payment: P,
    ) -> Tx<Env, From, To, MultiDcdtPayment<Env::Api>, Gas, Data, RH> {
        self.multi_dcdt(payment)
    }
}

impl<Env, From, To, Payment, Data, RH> Tx<Env, From, To, Payment, (), Data, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets an explicit gas limit to the call.
    #[inline]
    pub fn gas<GasValue>(
        self,
        gas_value: GasValue,
    ) -> Tx<Env, From, To, Payment, ExplicitGas<GasValue>, Data, RH>
    where
        GasValue: TxGasValue<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: ExplicitGas(gas_value),
            data: self.data,
            result_handler: self.result_handler,
        }
    }

    /// Backwards compatibility.
    #[inline]
    pub fn with_gas_limit(
        self,
        gas_limit: u64,
    ) -> Tx<Env, From, To, Payment, ExplicitGas<u64>, Data, RH> {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: ExplicitGas(gas_limit),
            data: self.data,
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, (), RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets the data field. Do not use directly.
    #[inline]
    #[doc(hidden)]
    pub fn raw_data<Data>(self, data: Data) -> Tx<Env, From, To, Payment, Gas, Data, RH>
    where
        Data: TxData<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data,
            result_handler: self.result_handler,
        }
    }

    /// Starts a contract call, serialized by hand.
    ///
    /// Whenever possible, should use proxies instead, since manual serialization is not type-safe.
    #[inline]
    pub fn raw_call<N: Into<ManagedBuffer<Env::Api>>>(
        self,
        function_name: N,
    ) -> Tx<Env, From, To, Payment, Gas, FunctionCall<Env::Api>, RH> {
        self.raw_data(FunctionCall::new(function_name))
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, FunctionCall<Env::Api>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Converts tx to a simple FunctionCall, to be used as argument or data in contracts.
    pub fn into_function_call(self) -> FunctionCall<Env::Api> {
        self.data
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, FunctionCall<Env::Api>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxToSpecified<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Produces the normalized function call, i.e. with builtin function calls for DCDT transfers.
    ///
    /// The resulting transaction can differ from the input in several ways:
    /// - the recipient is changed (some builtin functions are called with recipient = sender),
    /// - the function call becomes a builtin function call.
    ///
    /// ## Important
    ///
    /// Do not call this before sending transactions! Normalization is don automatically whenever necessary.
    /// Only use when you need the normalized data, e.g. for a multisig.
    ///
    /// ## Warning
    ///
    /// To produce owned values, some clones are performed.
    /// It is not optimized for contracts, but can be used nonetheless.
    #[allow(clippy::type_complexity)]
    pub fn normalize(
        self,
    ) -> Tx<
        Env,
        From,
        ManagedAddress<Env::Api>,
        MoaPayment<Env::Api>,
        Gas,
        FunctionCall<Env::Api>,
        RH,
    > {
        let (norm_to, norm_moa, norm_fc) = self.payment.with_normalized(
            &self.env,
            &self.from,
            self.to,
            self.data,
            |norm_to, norm_moa, norm_fc| (norm_to.clone(), norm_moa.clone(), norm_fc),
        );

        Tx {
            env: self.env,
            from: self.from,
            to: norm_to,
            payment: Moa(norm_moa),
            gas: self.gas,
            data: norm_fc,
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, Payment, Gas> Tx<Env, From, (), Payment, Gas, (), ()>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
{
    /// Merges the argument data into the current tx.
    /// Used for function calls originating in legacy proxies.
    ///
    /// Different environment in the argument allowed because of compatibility with old proxies.
    ///
    /// Method still subject to considerable change.
    pub fn legacy_proxy_call<Env2, To, O>(
        self,
        call: Tx<Env2, (), To, (), (), FunctionCall<Env::Api>, OriginalResultMarker<O>>,
    ) -> Tx<Env, From, To, Payment, Gas, FunctionCall<Env::Api>, OriginalResultMarker<O>>
    where
        Env2: TxEnv<Api = Env::Api>,
        To: TxTo<Env> + TxTo<Env2>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: call.to,
            payment: self.payment,
            gas: self.gas,
            data: call.data,
            result_handler: call.result_handler,
        }
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, FunctionCall<Env::Api>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Adds argument to function call.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// It serializes the value, but does not enforce type safety.
    #[inline]
    pub fn argument<T: TopEncodeMulti>(mut self, arg: &T) -> Self {
        self.data = self.data.argument(arg);
        self
    }

    /// Adds serialized argument to function call.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// Doesa not serialize, does not enforce type safety.
    #[inline]
    pub fn arguments_raw(mut self, raw: ManagedArgBuffer<Env::Api>) -> Self {
        self.data.arg_buffer = raw;
        self
    }
}

impl<Env, From, To, Payment, Gas, Data> Tx<Env, From, To, Payment, Gas, Data, ()>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
{
    /// Type marker to set the original contract or VM function return type.
    ///
    /// Only the compile-time type annotation is given.
    #[inline]
    pub fn original_result<OriginalResult>(
        self,
    ) -> Tx<Env, From, To, Payment, Gas, Data, OriginalResultMarker<OriginalResult>> {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data,
            result_handler: OriginalResultMarker::new(),
        }
    }
}

impl<Env, From, To, Gas> Tx<Env, From, To, (), Gas, (), ()>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    /// Starts a proxy call, deploy, or upgrade.
    ///
    /// The proxy object will be given, the subsequent call will be from a proxy context, containing all the contract endpoint names.
    pub fn typed<Proxy>(self, proxy: Proxy) -> Proxy::TxProxyMethods
    where
        Proxy: TxProxyTrait<Env, From, To, Gas>,
    {
        proxy.proxy_methods(self)
    }
}

impl<Env, From, To, Payment, Gas, Data, ResultList>
    Tx<Env, From, To, Payment, Gas, Data, ResultList>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    ResultList: RHList<Env>,
{
    /// Adds a result handler that doesn't return anything.
    #[inline]
    pub fn with_result<ResultHandler>(
        self,
        result_handler: ResultHandler,
    ) -> Tx<Env, From, To, Payment, Gas, Data, ResultList::NoRetOutput>
    where
        ResultHandler: RHListItem<Env, ResultList::OriginalResult, Returns = ()>,
        ResultList: RHListAppendNoRet<Env, ResultHandler>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler.append_no_ret(result_handler),
        }
    }

    /// Adds a result handler that can also return processed data.
    #[inline]
    pub fn returns<RH>(
        self,
        item: RH,
    ) -> Tx<Env, From, To, Payment, Gas, Data, ResultList::RetOutput>
    where
        RH: RHListItem<Env, ResultList::OriginalResult>,
        ResultList: RHListAppendRet<Env, RH>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data,
            result_handler: self.result_handler.append_ret(item),
        }
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, (), RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Starts a contract deploy call, serialized by hand.
    ///
    /// Whenever possible, should use proxies instead, since manual serialization is not type-safe.
    pub fn raw_deploy(self) -> Tx<Env, From, To, Payment, Gas, DeployCall<Env, ()>, RH> {
        self.raw_data(DeployCall::default())
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, UpgradeCall<Env, ()>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets upgrade code source as explicit code bytes.
    pub fn code<CodeValue>(
        self,
        code: CodeValue,
    ) -> Tx<Env, From, To, Payment, Gas, UpgradeCall<Env, Code<CodeValue>>, RH>
    where
        CodeValue: TxCodeValue<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data.code_source(Code(code)),
            result_handler: self.result_handler,
        }
    }

    /// Sets upgrade code source as another deployed contract code.
    pub fn from_source<FromSourceValue>(
        self,
        source_address: FromSourceValue,
    ) -> Tx<Env, From, To, Payment, Gas, UpgradeCall<Env, FromSource<FromSourceValue>>, RH>
    where
        FromSourceValue: TxFromSourceValue<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data.code_source(FromSource(source_address)),
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, DeployCall<Env, ()>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets deploy code source as explicit code bytes.
    pub fn code<CodeValue>(
        self,
        code: CodeValue,
    ) -> Tx<Env, From, To, Payment, Gas, DeployCall<Env, Code<CodeValue>>, RH>
    where
        CodeValue: TxCodeValue<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data.code_source(Code(code)),
            result_handler: self.result_handler,
        }
    }

    /// Sets deploy code source as another deployed contract code.
    pub fn from_source<FromSourceValue>(
        self,
        source_address: FromSourceValue,
    ) -> Tx<Env, From, To, Payment, Gas, DeployCall<Env, FromSource<FromSourceValue>>, RH>
    where
        FromSourceValue: TxFromSourceValue<Env>,
    {
        Tx {
            env: self.env,
            from: self.from,
            to: self.to,
            payment: self.payment,
            gas: self.gas,
            data: self.data.code_source(FromSource(source_address)),
            result_handler: self.result_handler,
        }
    }
}

impl<Env, From, To, Payment, Gas, CodeSource, RH>
    Tx<Env, From, To, Payment, Gas, DeployCall<Env, CodeSource>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    CodeSource: TxCodeSource<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets code metadata to deploy.
    pub fn code_metadata(mut self, code_metadata: CodeMetadata) -> Self {
        self.data = self.data.code_metadata(code_metadata);
        self
    }

    /// Adds argument to a contract deploy.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// It serializes the value, but does not enforce type safety.
    #[inline]
    pub fn argument<T: TopEncodeMulti>(mut self, arg: &T) -> Self {
        self.data = self.data.argument(arg);
        self
    }

    /// Adds serialized argument to a contract deploy.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// Does not serialize, does not enforce type safety.
    #[inline]
    pub fn arguments_raw(mut self, raw: ManagedArgBuffer<Env::Api>) -> Self {
        self.data.arg_buffer = raw;
        self
    }
}

impl<Env, From, To, Payment, Gas, CodeSource, RH>
    Tx<Env, From, To, Payment, Gas, DeployCall<Env, CodeSource>, RH>
where
    Env: TxEnvMockDeployAddress,
    From: TxFromSpecified<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    CodeSource: TxCodeSource<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets the new mock address to be used for the newly deployed contract.
    ///
    /// Only allowed in tests.
    pub fn new_address<NA>(mut self, new_address: NA) -> Self
    where
        NA: AnnotatedValue<Env, ManagedAddress<Env::Api>>,
    {
        self.env.mock_deploy_new_address(&self.from, new_address);
        self
    }
}

impl<Env, From, To, Payment, Gas, RH> Tx<Env, From, To, Payment, Gas, (), RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    RH: TxResultHandler<Env>,
{
    /// Starts a contract deploy upgrade, serialized by hand.
    ///
    /// Whenever possible, should use proxies instead, since manual serialization is not type-safe.
    pub fn raw_upgrade(self) -> Tx<Env, From, To, Payment, Gas, UpgradeCall<Env, ()>, RH> {
        self.raw_data(UpgradeCall::default())
    }
}

impl<Env, From, To, Payment, Gas, CodeSource, RH>
    Tx<Env, From, To, Payment, Gas, UpgradeCall<Env, CodeSource>, RH>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPaymentMoaOnly<Env>,
    Gas: TxGas<Env>,
    CodeSource: TxCodeSource<Env>,
    RH: TxResultHandler<Env>,
{
    pub fn code_metadata(mut self, code_metadata: CodeMetadata) -> Self {
        self.data = self.data.code_metadata(code_metadata);
        self
    }

    /// Adds argument to upgrade call.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// It serializes the value, but does not enforce type safety.
    #[inline]
    pub fn argument<T: TopEncodeMulti>(mut self, arg: &T) -> Self {
        self.data = self.data.argument(arg);
        self
    }

    /// Adds serialized argument to an upgrade call.
    ///
    /// Whenever possible, use proxies instead.
    ///
    /// Doesa not serialize, does not enforce type safety.
    #[inline]
    pub fn arguments_raw(mut self, raw: ManagedArgBuffer<Env::Api>) -> Self {
        self.data.arg_buffer = raw;
        self
    }
}

impl<Env, From, To, Payment, Gas, Data, RH> Tx<Env, From, To, Payment, Gas, Data, RH>
where
    Env: TxEnvWithTxHash,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Payment: TxPayment<Env>,
    Gas: TxGas<Env>,
    Data: TxData<Env>,
    RH: TxResultHandler<Env>,
{
    /// Sets the mock transaction hash to be used in a test.
    ///
    /// Only allowed in tests.
    pub fn tx_hash<H>(mut self, tx_hash: H) -> Self
    where
        H256: core::convert::From<H>,
    {
        self.env.set_tx_hash(H256::from(tx_hash));
        self
    }
}

impl<Api, To, Payment, OriginalResult>
    From<
        Tx<
            TxScEnv<Api>,
            (),
            To,
            Payment,
            (),
            DeployCall<TxScEnv<Api>, ()>,
            OriginalResultMarker<OriginalResult>,
        >,
    > for ContractDeploy<Api, OriginalResult>
where
    Api: CallTypeApi + 'static,
    To: TxTo<TxScEnv<Api>>,
    Payment: TxPaymentMoaOnly<TxScEnv<Api>>,
    OriginalResult: TopEncodeMulti,
{
    fn from(
        value: Tx<
            TxScEnv<Api>,
            (),
            To,
            Payment,
            (),
            DeployCall<TxScEnv<Api>, ()>,
            OriginalResultMarker<OriginalResult>,
        >,
    ) -> Self {
        ContractDeploy {
            _phantom: core::marker::PhantomData,
            to: ManagedOption::none(),
            moa_payment: value.payment.into_moa_payment(&value.env),
            explicit_gas_limit: UNSPECIFIED_GAS_LIMIT,
            arg_buffer: value.data.arg_buffer,
            _return_type: core::marker::PhantomData,
        }
    }
}

// Conversion from new syntax to old syntax.
impl<Api, To, Payment, OriginalResult> ContractCallBase<Api>
    for Tx<
        TxScEnv<Api>,
        (),
        To,
        Payment,
        (),
        FunctionCall<Api>,
        OriginalResultMarker<OriginalResult>,
    >
where
    Api: CallTypeApi + 'static,
    To: TxToSpecified<TxScEnv<Api>>,
    Payment: TxPayment<TxScEnv<Api>>,
    OriginalResult: TopEncodeMulti,
{
    type OriginalResult = OriginalResult;

    fn into_normalized(self) -> ContractCallWithMoa<Api, OriginalResult> {
        self.payment.with_normalized(
            &self.env,
            &self.from,
            self.to,
            self.data,
            |norm_to, norm_moa, norm_fc| ContractCallWithMoa {
                basic: ContractCallNoPayment {
                    _phantom: core::marker::PhantomData,
                    to: norm_to.clone(),
                    function_call: norm_fc.clone(),
                    explicit_gas_limit: UNSPECIFIED_GAS_LIMIT,
                    _return_type: core::marker::PhantomData,
                },
                moa_payment: norm_moa.clone(),
            },
        )
    }
}
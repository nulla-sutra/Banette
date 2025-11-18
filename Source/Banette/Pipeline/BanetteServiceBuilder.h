// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Core/BanetteService.h"
#include "Core/BanetteLayer.h"

namespace Banette::Builder
{
	using namespace Banette::Core;

	/// 空状态：还没有 Service
	class FEmptyServiceState
	{
	public:
		using ServiceType = void;
	};

	/// ServiceBuilder：链式构建一个被多个 Layer 包装的 Service
	template <typename CurrentServiceT = void>
	class BANETTE_API TServiceBuilder
	{
	public:
		TServiceBuilder() = default;

		/// 从一个具体的 Service 开始
		template <CService S>
		static TServiceBuilder<S> New(TSharedRef<S> InService)
		{
			return TServiceBuilder<S>(InService);
		}

		/// 应用一个 Layer：把当前 Service 传给 Layer，得到一个新的 Service
		/// Layer 必须满足：它的 InServiceType 是 CurrentServiceT，OutServiceType 是新类型
		template <CLayer L>
			requires std::is_same_v<typename L::InServiceType, CurrentServiceT>
		TServiceBuilder<typename L::OutServiceType> Layer(L& InLayer)
		{
			static_assert(!std::is_same_v<CurrentServiceT, void>,
			              "Cannot apply layer to empty builder. Start with New(service) first.");

			auto WrappedService = InLayer.Wrap(CurrentService);
			return TServiceBuilder<typename L::OutServiceType>(WrappedService);
		}

		/// 获取最终的 Service
		template <typename S = CurrentServiceT>
			requires (!std::is_same_v<S, void>)
		TSharedRef<S> Build() const
		{
			static_assert(!std::is_same_v<CurrentServiceT, void>,
			              "Cannot build from empty builder. Start with New(service) first.");
			return CurrentService;
		}

	private:
		explicit TServiceBuilder(TSharedRef<CurrentServiceT> InService)
			: CurrentService(InService)
		{
		}

		TSharedRef<CurrentServiceT> CurrentService;
	};

	/// 特化版本：空状态
	template <>
	class TServiceBuilder<void>
	{
	public:
		TServiceBuilder() = default;

		/// 从一个具体的 Service 开始
		template <CService S>
		static TServiceBuilder<S> New(TSharedRef<S> InService)
		{
			return TServiceBuilder<S>(InService);
		}

		template <typename S>
		TServiceBuilder<S> New(TSharedRef<S> InService)
		{
			return TServiceBuilder<S>(InService);
		}
	};
}

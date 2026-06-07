import React from 'react';
import { ClickUI, ThemeModeType } from '@make-software/csprclick-ui';

import { accountMenuItems } from './settings/account-menu';
import { TopBarContainer, TopBarSection } from './styled';

export interface TopBarProps {
  themeMode: ThemeModeType | undefined;
  onThemeSwitch: () => void;
}

export const ClickTopBar: React.FC<TopBarProps> = ({ themeMode, onThemeSwitch }) => {


  return (
    <TopBarSection>
      <TopBarContainer>
        <ClickUI
          topBarSettings={{
            onThemeSwitch: onThemeSwitch,
            accountMenuItems: accountMenuItems
          }}
          themeMode={themeMode}
        />
      </TopBarContainer>
    </TopBarSection>
  );
};
